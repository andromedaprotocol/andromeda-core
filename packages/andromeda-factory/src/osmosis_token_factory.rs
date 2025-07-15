use andromeda_std::{
    andr_exec, andr_instantiate, andr_query
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Receive CW20 tokens and lock them, minting factory tokens
    Receive(Cw20ReceiveMsg),
    /// Unlock CW20 tokens by burning factory tokens
    Unlock {
        cw20_addr: Addr,
        factory_denom: String,
        amount: Uint128,
    },
}

#[cw_serde]
pub enum ReceiveHook {
    /// Lock the received CW20 tokens and mint factory tokens
    Lock {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the locked amount for a specific owner and CW20 token
    #[returns(LockedResponse)]
    Locked { 
        owner: Addr, 
        cw20_addr: Addr 
    },
    /// Get the factory denom for a CW20 token
    #[returns(FactoryDenomResponse)]
    FactoryDenom { 
        cw20_addr: Addr 
    },
    /// Get all locked tokens for an owner
    #[returns(AllLockedResponse)]
    AllLocked { 
        owner: Addr 
    },
}

#[cw_serde]
pub struct LockedResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct FactoryDenomResponse {
    pub factory_denom: Option<String>,
}

#[cw_serde]
pub struct AllLockedResponse {
    pub locked: Vec<LockedToken>,
}

#[cw_serde]
pub struct LockedToken {
    pub cw20_addr: Addr,
    pub amount: Uint128,
    pub factory_denom: String,
} 