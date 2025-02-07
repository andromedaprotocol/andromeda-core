use andromeda_finance::mint_burn::{
    ExecuteMsg, GetOrderInfoResponse, GetOrdersByStatusResponse, GetUserDepositedOrdersReponse,
    InstantiateMsg, OrderStatus, QueryMsg, Resource, ResourceRequirement,
};
use andromeda_std::{
    amp::AndrAddr,
    common::{
        denom::{AuthorizedAddressesResponse, PermissionAction},
        OrderBy,
    },
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, DepsMut, MessageInfo, OwnedDeps, Response, Uint128,
};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;

use crate::contract::{execute, instantiate, query};

pub const MOCK_CW20_CONTRACT_2: &str = "cw20_contract_2";
pub const MOCK_NFT_CONTRACT: &str = "nft_contract";
pub const MOCK_NFT_CONTRACT_TO_MINT: &str = "nft_contract_to_mint";

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(
    authorized_nft_addresses: Option<Vec<AndrAddr>>,
    authorized_cw20_addresses: Option<Vec<AndrAddr>>,
) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        authorized_nft_addresses,
        authorized_cw20_addresses,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn create_order(
    deps: DepsMut<'_>,
    requirements: Vec<ResourceRequirement>,
    output: Resource,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::CreateOrder {
        requirements,
        output,
    };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn cancel_order(
    deps: DepsMut<'_>,
    order_id: Uint128,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::CancelOrder { order_id };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn receive_cw20(
    deps: DepsMut<'_>,
    cw20_receive_msg: Cw20ReceiveMsg,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::ReceiveCw20(cw20_receive_msg);
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn receive_nft(
    deps: DepsMut<'_>,
    nft_receive_msg: Cw721ReceiveMsg,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::ReceiveNft(nft_receive_msg);
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_order_info(
    deps: Deps,
    order_id: Uint128,
) -> Result<GetOrderInfoResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetOrderInfo { order_id });
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_orders_by_status(
    deps: Deps,
    status: OrderStatus,
    limit: Option<Uint128>,
) -> Result<GetOrdersByStatusResponse, ContractError> {
    let res = query(
        deps,
        mock_env(),
        QueryMsg::GetOrdersByStatus { status, limit },
    );
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_user_deposited_orders(
    deps: Deps,
    user: AndrAddr,
    limit: Option<Uint128>,
) -> Result<GetUserDepositedOrdersReponse, ContractError> {
    let res = query(
        deps,
        mock_env(),
        QueryMsg::GetUserDepositedOrders { user, limit },
    );
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_authorized_addresses(
    deps: Deps,
    action: PermissionAction,
    start_after: Option<String>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> Result<AuthorizedAddressesResponse, ContractError> {
    let res = query(
        deps,
        mock_env(),
        QueryMsg::AuthorizedAddresses {
            action,
            start_after,
            limit,
            order_by,
        },
    );
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
