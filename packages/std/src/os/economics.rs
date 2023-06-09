use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::amp::AndrAddr;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the Kernel contract on chain
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Deposit {
        address: Option<AndrAddr>,
    },
    PayFee {
        payee: Addr,
        action: String,
    },
    Withdraw {
        amount: Option<Uint128>,
        asset: String,
    },
    #[serde(rename = "withdraw_cw20")]
    WithdrawCW20 {
        amount: Option<Uint128>,
        asset: String,
    },
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum Cw20HookMsg {
    Deposit { address: Option<AndrAddr> },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cfg(test)]
mod test {}
