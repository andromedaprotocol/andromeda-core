use andromeda_std::andr_execute_fn;

use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    common::{context::ExecuteContext, denom::Asset, encode_binary},
    error::ContractError,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    attr, entry_point, from_json, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError,
};

use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;

use crate::astroport::ASTROPORT_MSG_WITHDRAW_LIQUIDITY_ID;
use crate::execute::{
    create_factory_pair, create_pair_and_provide_liquidity, execute_swap_and_forward,
    execute_update_swap_router, provide_liquidity, swap_and_forward_cw20, withdraw_liquidity,
};
use crate::query::{
    query_lp_pair_address, query_pair_address, query_simulate_astro_swap_operation,
};
use crate::reply::{
    check_reply_result, handle_astroport_create_pair_and_provide_liquidity_reply,
    handle_astroport_create_pair_reply, handle_astroport_withdraw_liquidity_reply,
};
use crate::{
    astroport::{
        ASTROPORT_MSG_CREATE_PAIR_AND_PROVIDE_LIQUIDITY_ID, ASTROPORT_MSG_CREATE_PAIR_ID,
        ASTROPORT_MSG_FORWARD_ID, ASTROPORT_MSG_PROVIDE_LIQUIDITY_ID, ASTROPORT_MSG_SWAP_ID,
    },
    reply::handle_astroport_swap_reply,
    state::{ForwardReplyState, FACTORY, FORWARD_REPLY_STATE, SWAP_ROUTER},
};

use andromeda_socket::astroport::{
    AssetEntry, AssetInfo, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};

const CONTRACT_NAME: &str = "crates.io:andromeda-socket-astroport";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address.clone(),
            owner: msg.owner,
        },
    )?;

    let swap_router = msg
        .swap_router
        .unwrap_or(AndrAddr::from_string("/lib/astroport/router"));

    swap_router.get_raw_address(&deps.as_ref())?;
    SWAP_ROUTER.save(deps.storage, &swap_router)?;

    let factory = msg
        .factory
        .unwrap_or(AndrAddr::from_string("/lib/astroport/factory"));
    let factory_raw_address = factory.get_raw_address(&deps.as_ref())?;
    FACTORY.save(deps.storage, &factory_raw_address.into())?;

    Ok(inst_resp
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => handle_receive_cw20(ctx, msg),
        ExecuteMsg::SwapAndForward {
            to_asset,
            recipient,
            max_spread,
            minimum_receive,
            operations,
        } => execute_swap_and_forward(
            ctx,
            to_asset,
            recipient,
            max_spread,
            minimum_receive,
            operations,
        ),
        ExecuteMsg::UpdateSwapRouter { swap_router } => {
            execute_update_swap_router(ctx, swap_router)
        }
        ExecuteMsg::CreatePair {
            pair_type,
            asset_infos,
            init_params,
        } => create_factory_pair(ctx, pair_type, asset_infos, init_params),
        ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            auto_stake,
            receiver,
        } => provide_liquidity(ctx, assets, slippage_tolerance, auto_stake, receiver),
        ExecuteMsg::CreatePairAndProvideLiquidity {
            pair_type,
            asset_infos,
            init_params,
            assets,
            slippage_tolerance,
            auto_stake,
            receiver,
        } => create_pair_and_provide_liquidity(
            ctx,
            pair_type,
            asset_infos,
            init_params,
            assets,
            slippage_tolerance,
            auto_stake,
            receiver,
        ),
        ExecuteMsg::WithdrawLiquidity {} => withdraw_liquidity(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }
}
fn handle_receive_cw20(
    ctx: ExecuteContext,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let ExecuteContext { ref info, .. } = ctx;

    let amount = cw20_msg.amount;
    let sender = cw20_msg.sender;
    let from_addr = AndrAddr::from_string(info.sender.clone());
    let from_asset = Asset::Cw20Token(from_addr.clone());

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::SwapAndForward {
            to_asset,
            recipient,
            max_spread,
            minimum_receive,
            operations,
        } => {
            let recipient = match recipient {
                None => Recipient::new(sender.clone(), None),
                Some(recipient) => recipient,
            };
            recipient.validate(&ctx.deps.as_ref())?;

            swap_and_forward_cw20(
                ctx,
                from_asset,
                amount,
                to_asset,
                recipient,
                AndrAddr::from_string(sender),
                max_spread,
                minimum_receive,
                operations,
            )
        }
        Cw20HookMsg::ProvideLiquidity {
            other_asset,
            slippage_tolerance,
            auto_stake,
            receiver,
        } => {
            let cw20_asset = AssetEntry {
                info: AssetInfo::Token {
                    contract_addr: info.sender.clone(),
                },
                amount,
            };

            let assets = vec![cw20_asset, other_asset];

            provide_liquidity(ctx, assets, slippage_tolerance, auto_stake, receiver)
        }
        Cw20HookMsg::CreatePairAndProvideLiquidity {
            pair_type,
            asset_infos,
            init_params,
            other_asset,
            slippage_tolerance,
            auto_stake,
            receiver,
        } => {
            let cw20_asset = AssetEntry {
                info: AssetInfo::Token {
                    contract_addr: info.sender.clone(),
                },
                amount,
            };

            let assets = vec![cw20_asset, other_asset];

            create_pair_and_provide_liquidity(
                ctx,
                pair_type,
                asset_infos,
                init_params,
                assets,
                slippage_tolerance,
                auto_stake,
                receiver,
            )
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::SimulateSwapOperation {
            offer_amount,
            operations,
        } => encode_binary(&query_simulate_astro_swap_operation(
            deps,
            offer_amount,
            operations,
        )?),
        QueryMsg::PairAddress {} => encode_binary(&query_pair_address(deps)?),
        QueryMsg::LpPairAddress {} => encode_binary(&query_lp_pair_address(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        ASTROPORT_MSG_SWAP_ID => {
            let state: ForwardReplyState = FORWARD_REPLY_STATE.load(deps.storage)?;
            FORWARD_REPLY_STATE.remove(deps.storage);
            check_reply_result(&msg, "swap")?;
            handle_astroport_swap_reply(deps, env, msg, state)
        }
        ASTROPORT_MSG_FORWARD_ID => {
            check_reply_result(&msg, "msg forwarding")?;
            Ok(Response::default()
                .add_attributes(vec![attr("action", "message_forwarded_success")]))
        }
        ASTROPORT_MSG_CREATE_PAIR_ID => {
            check_reply_result(&msg, "create pair")?;
            handle_astroport_create_pair_reply(deps, msg)
        }
        ASTROPORT_MSG_CREATE_PAIR_AND_PROVIDE_LIQUIDITY_ID => {
            check_reply_result(&msg, "create pair and provide liquidity")?;
            handle_astroport_create_pair_and_provide_liquidity_reply(deps, msg)
        }
        ASTROPORT_MSG_PROVIDE_LIQUIDITY_ID => {
            check_reply_result(&msg, "provide liquidity")?;
            Ok(Response::default()
                .add_attributes(vec![attr("action", "provide_liquidity_success")]))
        }
        ASTROPORT_MSG_WITHDRAW_LIQUIDITY_ID => {
            check_reply_result(&msg, "withdraw liquidity")?;
            handle_astroport_withdraw_liquidity_reply(deps, msg)
        }
        _ => Err(ContractError::Std(StdError::generic_err(
            "Invalid Reply ID".to_string(),
        ))),
    }
}
