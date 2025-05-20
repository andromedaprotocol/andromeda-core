use andromeda_std::andr_execute_fn;

use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    common::{context::ExecuteContext, denom::Asset, encode_binary},
    error::ContractError,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_json, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_utils::one_coin;

use crate::{
    astroport::{
        execute_swap_astroport_msg, handle_astroport_swap_reply,
        query_simulate_astro_swap_operation, ASTROPORT_MSG_FORWARD_ID, ASTROPORT_MSG_SWAP_ID,
    },
    state::{ForwardReplyState, FORWARD_REPLY_STATE, SWAP_ROUTER},
};

use andromeda_socket::astroport::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, SimulateSwapOperationResponse, SwapOperation,
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
    let from_asset = Asset::Cw20Token(from_addr);

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
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_swap_and_forward(
    ctx: ExecuteContext,
    to_asset: Asset,
    recipient: Option<Recipient>,
    max_spread: Option<Decimal>,
    minimum_receive: Option<Uint128>,
    operations: Option<Vec<SwapOperation>>,
) -> Result<Response, ContractError> {
    let fund = one_coin(&ctx.info).map_err(|_| ContractError::InvalidAsset {
        asset: "Invalid or missing coin".to_string(),
    })?;

    let from_asset = Asset::NativeToken(fund.denom);
    let sender = AndrAddr::from_string(&ctx.info.sender);
    let recipient = match recipient {
        None => Recipient::new(sender.clone(), None),
        Some(recipient) => recipient,
    };
    recipient.validate(&ctx.deps.as_ref())?;

    let swap_msg = execute_swap_astroport_msg(
        ctx,
        from_asset.clone(),
        fund.amount,
        to_asset.clone(),
        recipient.clone(),
        sender,
        max_spread,
        minimum_receive,
        operations,
    )?;

    Ok(Response::default()
        .add_submessage(swap_msg)
        .add_attributes(vec![
            attr("from_asset", from_asset.to_string()),
            attr("from_amount", fund.amount),
            attr("to_asset", to_asset.to_string()),
            attr("recipient", recipient.get_addr()),
        ]))
}

#[allow(clippy::too_many_arguments)]
fn swap_and_forward_cw20(
    ctx: ExecuteContext,
    from_asset: Asset,
    from_amount: Uint128,
    to_asset: Asset,
    recipient: Recipient,
    refund_addr: AndrAddr,
    max_spread: Option<Decimal>,
    minimum_receive: Option<Uint128>,
    operations: Option<Vec<SwapOperation>>,
) -> Result<Response, ContractError> {
    let swap_msg = execute_swap_astroport_msg(
        ctx,
        from_asset.clone(),
        from_amount,
        to_asset.clone(),
        recipient.clone(),
        refund_addr,
        max_spread,
        minimum_receive,
        operations,
    )?;

    Ok(Response::default()
        .add_submessage(swap_msg)
        .add_attributes(vec![
            attr("from_asset", from_asset.to_string()),
            attr("from_amount", from_amount),
            attr("to_asset", to_asset.to_string()),
            attr("recipient", recipient.get_addr()),
        ]))
}

fn execute_update_swap_router(
    ctx: ExecuteContext,
    swap_router: AndrAddr,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;

    // Verify sender has owner permissions
    ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?;

    swap_router.get_raw_address(&deps.as_ref())?;
    let previous_swap_router = SWAP_ROUTER.load(deps.storage)?;

    SWAP_ROUTER.save(deps.storage, &swap_router)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "update-swap-router"),
        attr("previous_swap_router", previous_swap_router),
        attr("swap_router", swap_router),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::SimulateSwapOperation {
            offer_amount,
            operations,
        } => encode_binary(&query_simulate_swap_operation(
            deps,
            offer_amount,
            operations,
        )?),
    }
}

fn query_simulate_swap_operation(
    deps: Deps,
    offer_amount: Uint128,
    swap_operation: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationResponse, ContractError> {
    query_simulate_astro_swap_operation(deps, offer_amount, swap_operation)
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

            if msg.result.is_err() {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "Astroport swap failed with error: {:?}",
                    msg.result.unwrap_err()
                ))))
            } else {
                handle_astroport_swap_reply(deps, env, msg, state)
            }
        }
        ASTROPORT_MSG_FORWARD_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Astroport msg forwarding failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }
            Ok(Response::default()
                .add_attributes(vec![attr("action", "message_forwarded_success")]))
        }
        _ => Err(ContractError::Std(StdError::generic_err(
            "Invalid Reply ID".to_string(),
        ))),
    }
}
