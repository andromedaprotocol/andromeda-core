use andromeda_std::{
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{
        denom::{PermissionAction, SEND_CW20_ACTION, SEND_NFT_ACTION},
        OrderBy,
    },
    error::ContractError,
};
use andromeda_std::{andr_exec, andr_instantiate, andr_query};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, DepsMut, Env, Uint128};
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

impl ExecuteMsg {
    // The maximum number of different resources that can be required in an order.
    const MAX_REQUIREMENTS: usize = 10;

    // Validates the `CreateOrder` message.
    pub fn requirements_number_validate(&self) -> Result<(), ContractError> {
        match self {
            ExecuteMsg::CreateOrder { requirements, .. } => {
                // Ensure that the number of requirements does not exceed the allowed limit
                ensure!(
                    requirements.len() <= Self::MAX_REQUIREMENTS,
                    ContractError::CustomError {
                        msg: format!(
                            "Too many requirements. Maximum allowed is {}.",
                            Self::MAX_REQUIREMENTS
                        )
                    }
                );
            }
            _ => {}
        }
        Ok(())
    }
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

// Represents a requirement for a specific resource (CW20 or CW721) in an order.
// This struct defines what must be deposited to fulfill an order.
#[cw_serde]
pub struct ResourceRequirement {
    // The type of resource required (CW20 token or CW721 NFT).
    pub resource: Resource,
    // The total amount of this resource required to complete the order.
    pub amount: Uint128,
    // A mapping of user addresses to the amount they have deposited towards this requirement.
    // - **Key:** User address (`String`)
    // - **Value:** Amount of the resource deposited (`Uint128`)
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
    pub fn validate(&self, mut deps: DepsMut, env: Env) -> Result<(), ContractError> {
        match &self.resource {
            Resource::Nft { cw721_addr, .. } => {
                ADOContract::default().is_permissioned(
                    deps,
                    env.clone(),
                    SEND_NFT_ACTION,
                    cw721_addr,
                )?;
                ensure!(
                    self.amount == Uint128::one(),
                    ContractError::CustomError {
                        msg: "Amount must be one if the resource is nft".to_string()
                    }
                );
                Ok(())
            }
            Resource::Cw20Token { cw20_addr } => {
                ADOContract::default().is_permissioned(
                    deps.branch(),
                    env.clone(),
                    SEND_CW20_ACTION,
                    cw20_addr,
                )?;
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
