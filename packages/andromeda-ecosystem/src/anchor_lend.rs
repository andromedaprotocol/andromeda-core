use common::ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    pub primitive_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    AndrReceive(AndromedaMsg),
    /// Deposit LUNA as collateral which will be converted to bLUNA.
    DepositCollateral {},
    /// Withdraw specified collateral. If unbond is true and collateral is bLuna, the unbonding
    /// process will begin, otherwise the collateral will be sent to the given recipient.
    WithdrawCollateral {
        collateral_addr: String,
        amount: Option<Uint256>,
        unbond: Option<bool>,
        recipient: Option<Recipient>,
    },
    /// Borrows funds to reach the desired loan-to-value ratio and sends the borrowed funds to the
    /// given recipient.
    Borrow {
        desired_ltv_ratio: Decimal256,
        recipient: Option<Recipient>,
    },
    /// Repays any existing loan with sent stable coins.
    RepayLoan {},
    /// Withdraws any unbonded bLuna from the hub contract.
    WithdrawUnbonded {
        recipient: Option<Recipient>,
    },
    /// Claims any outstanding ANC rewards with an option to stake them in governance.
    ClaimAncRewards {
        auto_stake: Option<bool>,
    },
    /// Stakes all or the specified amount of ANC tokens in the contract in governance.
    StakeAnc {
        amount: Option<Uint128>,
    },
    /// Unstakes all or the specified amount of ANC tokens in the contract in governance.
    UnstakeAnc {
        amount: Option<Uint128>,
    },

    /// INTERNAL
    DepositCollateralToAnchor {
        collateral_addr: String,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Deposit Cw20 assets as collateral.
    DepositCollateral {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
}

/* Begin BLunaHub enums and structs */

#[cw_serde]
pub enum BLunaHubExecuteMsg {
    Bond {},
    WithdrawUnbonded {},
}

#[cw_serde]
pub enum BLunaHubQueryMsg {
    WithdrawableUnbonded { address: String },
}

#[cw_serde]
pub struct WithdrawableUnbondedResponse {
    pub withdrawable: Uint128,
}

#[cw_serde]
pub enum BLunaHubCw20HookMsg {
    Unbond {},
}
