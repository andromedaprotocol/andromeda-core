#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, Uint128};

use andromeda_finance::mint_burn::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    andr_execute_fn,
    common::{
        denom::{authorize_addresses, SEND_CW20_ACTION, SEND_NFT_ACTION},
        encode_binary,
    },
    error::ContractError,
};

use crate::execute::{
    execute_cancel_order, execute_create_order, handle_receive_cw20, handle_receive_cw721,
};
use crate::query::{
    query_authorized_addresses, query_order_info, query_orders_by_status,
    query_user_deposited_orders,
};
use crate::state::NEXT_ORDER_ID;

// version info for migration info
pub const CONTRACT_NAME: &str = "crates.io:andromeda-mint-burn";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let resp = ADOContract::default().instantiate(
        deps.storage,
        env.clone(),
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    NEXT_ORDER_ID.save(deps.storage, &Uint128::from(1u128))?;

    if let Some(authorized_token_addresses) = msg.authorized_nft_addresses {
        authorize_addresses(&mut deps, SEND_NFT_ACTION, authorized_token_addresses)?;
    }

    if let Some(authorized_cw20_addresses) = msg.authorized_cw20_addresses {
        authorize_addresses(&mut deps, SEND_CW20_ACTION, authorized_cw20_addresses)?;
    }

    Ok(resp)
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateOrder {
            requirements,
            output,
        } => execute_create_order(ctx, requirements, output),
        ExecuteMsg::CancelOrder { order_id } => execute_cancel_order(ctx, order_id),
        ExecuteMsg::ReceiveNft(msg) => handle_receive_cw721(ctx, msg),
        ExecuteMsg::ReceiveCw20(msg) => handle_receive_cw20(ctx, msg),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetOrderInfo { order_id } => encode_binary(&query_order_info(deps, order_id)?),
        QueryMsg::GetOrdersByStatus { status, limit } => {
            encode_binary(&query_orders_by_status(deps, status, limit)?)
        }
        QueryMsg::GetUserDepositedOrders { user, limit } => {
            encode_binary(&query_user_deposited_orders(deps, user, limit)?)
        }
        QueryMsg::AuthorizedAddresses {
            action,
            start_after,
            limit,
            order_by,
        } => encode_binary(&query_authorized_addresses(
            deps,
            action,
            start_after,
            limit,
            order_by,
        )?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
}
