use andromeda_finance::mint_burn::{
    GetOrderInfoResponse, GetOrdersByStatusResponse, GetUserDepositedOrdersReponse, OrderInfo,
    OrderStatus,
};
use andromeda_std::{
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{
        denom::{AuthorizedAddressesResponse, PermissionAction},
        OrderBy,
    },
    error::ContractError,
};
use cosmwasm_std::{Deps, Order, StdResult, Uint128};

use crate::state::ORDERS;

const DEFAULT_LIMIT: Uint128 = Uint128::new(100);

pub fn query_order_info(
    deps: Deps,
    order_id: Uint128,
) -> Result<GetOrderInfoResponse, ContractError> {
    let order =
        ORDERS
            .load(deps.storage, order_id.u128())
            .map_err(|_| ContractError::CustomError {
                msg: "Not existed order".to_string(),
            })?;

    let response = GetOrderInfoResponse {
        order_id,
        requirements: order.requirements,
        output: order.output,
        order_status: order.status,
        output_recipient: order.output_recipient,
    };
    Ok(response)
}

pub fn query_orders_by_status(
    deps: Deps,
    status: OrderStatus,
    limit: Option<Uint128>,
) -> Result<GetOrdersByStatusResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).u128() as usize;

    let orders: Vec<(u128, OrderInfo)> = ORDERS
        .range(deps.storage, None, None, Order::Ascending)
        .filter(|res| match res {
            Ok((_, order)) => order.status == status,
            _ => false,
        })
        .take(limit)
        .collect::<StdResult<_>>()?;

    let mut res = Vec::new();

    for order in orders {
        let order_info = GetOrderInfoResponse {
            order_id: Uint128::new(order.0),
            requirements: order.1.requirements,
            output: order.1.output,
            order_status: order.1.status,
            output_recipient: order.1.output_recipient,
        };
        res.push(order_info);
    }

    Ok(GetOrdersByStatusResponse { orders: res })
}

pub fn query_user_deposited_orders(
    deps: Deps,
    user: AndrAddr,
    limit: Option<Uint128>,
) -> Result<GetUserDepositedOrdersReponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).u128() as usize;

    let mut res = Vec::new();

    let raw_user_str = user.get_raw_address(&deps)?.to_string();
    let orders: Vec<(u128, OrderInfo)> = ORDERS
        .range(deps.storage, None, None, Order::Ascending)
        .filter(|res| match res {
            Ok((_, order)) => order
                .requirements
                .iter()
                .any(|req| req.deposits.contains_key(&raw_user_str)),
            _ => false,
        })
        .take(limit)
        .collect::<StdResult<_>>()?;

    for order in orders {
        let order_info = GetOrderInfoResponse {
            order_id: Uint128::new(order.0),
            requirements: order.1.requirements,
            output: order.1.output,
            order_status: order.1.status,
            output_recipient: order.1.output_recipient,
        };
        res.push(order_info);
    }
    Ok(GetUserDepositedOrdersReponse { orders: res })
}

pub fn query_authorized_addresses(
    deps: Deps,
    action: PermissionAction,
    start_after: Option<String>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> Result<AuthorizedAddressesResponse, ContractError> {
    let addresses = ADOContract::default().query_permissioned_actors(
        deps,
        action.as_str(),
        start_after,
        limit,
        order_by,
    )?;
    Ok(AuthorizedAddressesResponse { addresses })
}
