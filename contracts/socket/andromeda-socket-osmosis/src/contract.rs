use andromeda_std::andr_execute_fn;

use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, SubMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;

use osmosis_std::types::osmosis::gamm::v1beta1::MsgExitPool;
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as OsmosisCoin,
    osmosis::{
        concentratedliquidity::poolmodel::concentrated::v1beta1::MsgCreateConcentratedPool,
        cosmwasmpool::v1beta1::MsgCreateCosmWasmPool,
        gamm::poolmodels::{
            balancer::v1beta1::MsgCreateBalancerPool, stableswap::v1beta1::MsgCreateStableswapPool,
        },
    },
};

use crate::osmosis::{
    OSMOSIS_MSG_CREATE_BALANCER_POOL_ID, OSMOSIS_MSG_CREATE_CONCENTRATED_POOL_ID,
    OSMOSIS_MSG_CREATE_COSM_WASM_POOL_ID, OSMOSIS_MSG_CREATE_STABLE_POOL_ID,
    OSMOSIS_MSG_WITHDRAW_POOL_ID,
};
use crate::state::{SPENDER, WITHDRAW};
use crate::{
    osmosis::{
        execute_swap_osmosis_msg, handle_osmosis_swap_reply, query_get_route,
        OSMOSIS_MSG_FORWARD_ID, OSMOSIS_MSG_SWAP_ID,
    },
    state::{ForwardReplyState, FORWARD_REPLY_STATE, SWAP_ROUTER},
};

use andromeda_socket::osmosis::{
    ExecuteMsg, InstantiateMsg, Pool, QueryMsg, Slippage, SwapAmountInRoute,
};

const CONTRACT_NAME: &str = "crates.io:andromeda-socket-osmosis";
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
        .unwrap_or(AndrAddr::from_string("/lib/osmosis/router"));
    swap_router.get_raw_address(&deps.as_ref())?;
    SWAP_ROUTER.save(deps.storage, &swap_router)?;

    Ok(inst_resp
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SwapAndForward {
            to_denom,
            recipient,
            slippage,
            route,
        } => execute_swap_and_forward(ctx, to_denom, recipient, slippage, route),
        ExecuteMsg::UpdateSwapRouter { swap_router } => {
            execute_update_swap_router(ctx, swap_router)
        }
        ExecuteMsg::CreatePool { pool_type } => execute_create_pool(ctx, pool_type),
        ExecuteMsg::WithdrawPool { withdraw_msg } => execute_withdraw_pool(ctx, withdraw_msg),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_swap_and_forward(
    ctx: ExecuteContext,
    to_denom: String,
    recipient: Option<Recipient>,
    slippage: Slippage,
    route: Option<Vec<SwapAmountInRoute>>,
) -> Result<Response, ContractError> {
    let fund = one_coin(&ctx.info).map_err(|_| ContractError::InvalidAsset {
        asset: "Invalid or missing coin".to_string(),
    })?;

    let from_denom = fund.denom;
    let recipient = match recipient {
        None => Recipient::new(AndrAddr::from_string(&ctx.info.sender), None),
        Some(recipient) => recipient,
    };
    recipient.validate(&ctx.deps.as_ref())?;

    let swap_msg = execute_swap_osmosis_msg(
        ctx,
        from_denom.clone(),
        fund.amount,
        to_denom.clone(),
        recipient.clone(),
        slippage,
        route,
    )?;

    Ok(Response::default()
        .add_submessage(swap_msg)
        .add_attributes(vec![
            attr("from_denom", from_denom),
            attr("from_amount", fund.amount),
            attr("to_denom", to_denom),
            attr("recipient", recipient.get_addr()),
        ]))
}

pub fn execute_create_pool(
    ctx: ExecuteContext,
    pool_type: Pool,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;
    let funds = info.funds.as_slice();
    if funds.len() != 2 {
        return Err(ContractError::InvalidAsset {
            asset: "Expected exactly 2 coins for pool creation".to_string(),
        });
    }
    let denom0 = &funds[0].denom;
    let amount0 = &funds[0].amount;
    let denom1 = &funds[1].denom;
    let amount1 = &funds[1].amount;

    let contract_address: String = env.contract.address.into();

    SPENDER.save(deps.storage, &info.sender.to_string())?;

    let msg: SubMsg = match pool_type {
        Pool::Balancer {
            pool_params,
            pool_assets,
        } => {
            let msg = MsgCreateBalancerPool {
                sender: contract_address.clone(),
                pool_params,
                pool_assets,
                future_pool_governor: contract_address,
            };
            let create_balancer_pool_msg: CosmosMsg = msg.into();

            SubMsg::reply_always(
                create_balancer_pool_msg,
                OSMOSIS_MSG_CREATE_BALANCER_POOL_ID,
            )
        }
        Pool::Stable {
            pool_params,
            scaling_factors,
        } => {
            let msg = MsgCreateStableswapPool {
                sender: contract_address.clone(),
                pool_params,
                initial_pool_liquidity: vec![
                    OsmosisCoin {
                        denom: denom0.clone(),
                        amount: amount0.to_string(),
                    },
                    OsmosisCoin {
                        denom: denom1.clone(),
                        amount: amount1.to_string(),
                    },
                ],
                scaling_factors,
                future_pool_governor: contract_address.clone(),
                scaling_factor_controller: contract_address.clone(),
            };
            SubMsg::reply_always(msg, OSMOSIS_MSG_CREATE_STABLE_POOL_ID)
        }
        Pool::Concentrated {
            tick_spacing,
            spread_factor,
        } => {
            let msg = MsgCreateConcentratedPool {
                sender: contract_address.clone(),
                denom0: denom0.clone(),
                denom1: denom1.clone(),
                tick_spacing,
                spread_factor,
            };
            SubMsg::reply_always(msg, OSMOSIS_MSG_CREATE_CONCENTRATED_POOL_ID)
        }
        Pool::CosmWasm {
            code_id,
            instantiate_msg,
        } => {
            let msg = MsgCreateCosmWasmPool {
                code_id,
                instantiate_msg,
                sender: contract_address.clone(),
            };
            SubMsg::reply_always(msg, OSMOSIS_MSG_CREATE_COSM_WASM_POOL_ID)
        }
    };

    Ok(Response::default().add_submessage(msg))
}

fn execute_withdraw_pool(
    ctx: ExecuteContext,
    withdraw_msg: MsgExitPool,
) -> Result<Response, ContractError> {
    let pool_id = WITHDRAW
        .may_load(ctx.deps.storage, ctx.info.sender.to_string())?
        .ok_or(ContractError::Std(StdError::generic_err(
            "Pool ID not found".to_string(),
        )))?
        .parse::<u64>()
        .map_err(|e| {
            ContractError::Std(StdError::generic_err(format!(
                "Failed to parse pool ID: {}",
                e
            )))
        })?;

    ensure!(
        pool_id == withdraw_msg.pool_id,
        ContractError::Std(StdError::generic_err("Pool ID mismatch".to_string(),))
    );

    let sub_msg = SubMsg::reply_always(withdraw_msg, OSMOSIS_MSG_WITHDRAW_POOL_ID);
    Ok(Response::default().add_submessage(sub_msg))
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
        QueryMsg::GetRoute {
            from_denom,
            to_denom,
        } => encode_binary(&query_get_route(deps, from_denom, to_denom)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        OSMOSIS_MSG_SWAP_ID => {
            let state: ForwardReplyState = FORWARD_REPLY_STATE.load(deps.storage)?;
            FORWARD_REPLY_STATE.remove(deps.storage);

            if msg.result.is_err() {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis swap failed with error: {:?}",
                    msg.result.unwrap_err()
                ))))
            } else {
                handle_osmosis_swap_reply(deps, env, msg, state)
            }
        }
        OSMOSIS_MSG_FORWARD_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis msg forwarding failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }
            Ok(Response::default()
                .add_attributes(vec![attr("action", "message_forwarded_success")]))
        }
        OSMOSIS_MSG_CREATE_BALANCER_POOL_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis balancer pool creation failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }
            // Query this contract's balances
            #[allow(deprecated)]
            let balances = deps.querier.query_all_balances(env.contract.address)?;
            // Extract the denom that contains "gamm/pool/"
            let lp_token = balances
                .iter()
                .find(|coin| coin.denom.contains("gamm/pool/"))
                .ok_or(ContractError::Std(StdError::generic_err(
                    "LP token not found".to_string(),
                )))?;

            let spender = SPENDER.load(deps.storage)?;
            // Tranfer lp token to original sender
            let msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: spender.clone(),
                amount: vec![lp_token.clone()],
            });

            let pool_id = lp_token
                .denom
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_string();

            WITHDRAW.save(deps.storage, spender.clone(), &pool_id)?;
            Ok(Response::default().add_message(msg).add_attributes(vec![
                attr("action", "balancer_pool_created"),
                attr("lp_token", lp_token.denom.clone()),
                attr("spender", spender),
                attr("amount", lp_token.amount.to_string()),
            ]))
        }
        OSMOSIS_MSG_CREATE_STABLE_POOL_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis stable pool creation failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }
            Ok(Response::default().add_attributes(vec![attr("action", "stable_pool_created")]))
        }
        OSMOSIS_MSG_CREATE_CONCENTRATED_POOL_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis concentrated pool creation failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }
            Ok(Response::default()
                .add_attributes(vec![attr("action", "concentrated_pool_created")]))
        }
        OSMOSIS_MSG_CREATE_COSM_WASM_POOL_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis cosmwasm pool creation failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }
            Ok(Response::default().add_attributes(vec![attr("action", "cosmwasm_pool_created")]))
        }
        OSMOSIS_MSG_WITHDRAW_POOL_ID => {
            if msg.result.is_err() {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis pool withdrawal failed with error: {:?}",
                    msg.result.unwrap_err()
                ))));
            }
            Ok(Response::default().add_attributes(vec![attr("action", "pool_withdrawn")]))
        }
        _ => Err(ContractError::Std(StdError::generic_err(
            "Invalid Reply ID".to_string(),
        ))),
    }
}
