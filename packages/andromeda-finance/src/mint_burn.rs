use andromeda_std::{
    amp::AndrAddr,
    common::{denom::PermissionAction, OrderBy},
    error::ContractError,
};
use andromeda_std::{andr_exec, andr_instantiate, andr_query};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Uint128};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use std::collections::HashMap;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub authorized_nft_addresses: Option<Vec<AndrAddr>>,
    pub authorized_cw20_addresses: Option<Vec<AndrAddr>>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    #[attrs(restricted)]
    CreateOrder {
        requirements: Vec<ResourceRequirement>,
        output: Resource,
    },
    #[attrs(restricted)]
    CancelOrder {
        order_id: Uint128,
    },
    ReceiveCw20(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
}

#[cw_serde]
pub struct OrderInfo {
    pub requirements: Vec<ResourceRequirement>,
    pub output: Resource,
    pub order_status: OrderStatus,
    pub output_recipient: Option<AndrAddr>,
}

#[cw_serde]
pub enum OrderStatus {
    NotCompleted,
    Completed,
    Cancelled,
}

#[cw_serde]
pub struct ResourceRequirement {
    pub resource: Resource,
    pub amount: Uint128,
    pub deposits: HashMap<String, Uint128>,
}

#[cw_serde]
pub enum Resource {
    Cw20Token {
        cw20_addr: AndrAddr,
    },
    Nft {
        cw721_addr: AndrAddr,
        token_id: String,
    },
}

#[cw_serde]
pub enum Cw721HookMsg {
    FillOrder { order_id: Uint128 },
}

#[cw_serde]
pub enum Cw20HookMsg {
    FillOrder { order_id: Uint128 },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetOrderInfoResponse)]
    GetOrderInfo { order_id: Uint128 },
    #[returns(GetOrdersByStatusResponse)]
    GetOrdersByStatus {
        status: OrderStatus,
        limit: Option<Uint128>,
    },
    #[returns(GetUserDepositedOrdersReponse)]
    GetUserDepositedOrders {
        user: AndrAddr,
        limit: Option<Uint128>,
    },
    #[returns(::andromeda_std::common::denom::AuthorizedAddressesResponse)]
    AuthorizedAddresses {
        action: PermissionAction,
        start_after: Option<String>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    },
}

#[cw_serde]
pub struct GetOrderInfoResponse {
    pub order_id: Uint128,
    pub requirements: Vec<ResourceRequirement>,
    pub output: Resource,
    pub order_status: OrderStatus,
    pub output_recipient: Option<AndrAddr>,
}

#[cw_serde]
pub struct GetOrdersByStatusResponse {
    pub orders: Vec<GetOrderInfoResponse>,
}

#[cw_serde]
pub struct GetUserDepositedOrdersReponse {
    pub orders: Vec<GetOrderInfoResponse>,
}

impl ResourceRequirement {
    pub fn validate(&self) -> Result<(), ContractError> {
        match self.resource {
            Resource::Nft { .. } => {
                ensure!(
                    self.amount == Uint128::one(),
                    ContractError::CustomError {
                        msg: "Amount must be one if the resource is nft".to_string()
                    }
                );
                Ok(())
            }
            Resource::Cw20Token { .. } => {
                ensure!(
                    self.amount != Uint128::zero(),
                    ContractError::CustomError {
                        msg: "Amount must not be zero if the resource is cw20".to_string()
                    }
                );
                Ok(())
            }
        }
    }
}
