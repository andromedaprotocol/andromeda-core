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
    /// Deposit funds to be used by the Andromeda economics module to pay for ADO fees.
    ///
    /// An optional valid VFS path can be provided to deposit funds on behalf of another address.
    Deposit {
        address: Option<AndrAddr>,
    },
    /// Pay a fee for the given action. The sender must be a valid ADO contract.
    ///
    /// Fees are paid in the following fallthrough priority:
    /// 1. The balance of the ADO contract itself
    /// 2. The balance of the App contract for the ADO
    /// 3. The provided payee address
    PayFee {
        payee: Addr,
        action: String,
    },
    /// Withdraw native funds from the Andromeda economics module.
    ///
    /// If no amount is provided all funds are withdrawn for the given asset.
    Withdraw {
        amount: Option<Uint128>,
        asset: String,
    },
    #[serde(rename = "withdraw_cw20")]
    /// Withdraw CW20 funds from the Andromeda economics module.
    ///
    /// If no amount is provided all funds are withdrawn for the given asset.
    WithdrawCW20 {
        amount: Option<Uint128>,
        asset: String,
    },
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Deposit CW20 tokens for use in paying fees
    ///
    /// An optional valid VFS path can be provided in order to deposit on behalf of another address.
    Deposit { address: Option<AndrAddr> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Queries the current balance for a given AndrAddr and asset tuple
    ///
    /// Returns a `Uint128` representing the current balance
    #[returns(BalanceResponse)]
    Balance { asset: String, address: AndrAddr },
}

#[cw_serde]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[cfg(test)]
mod test {}
