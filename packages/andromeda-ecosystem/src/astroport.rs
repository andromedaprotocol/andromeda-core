use crate::swapper::{SwapperCw20HookMsg, SwapperMsg};
use astroport::factory::ExecuteMsg as AstroportFactoryExecuteMsg;
use common::ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetUnchecked;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    pub primitive_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    Swapper(SwapperMsg),
    Receive(Cw20ReceiveMsg),
    AstroportFactoryExecuteMsg(AstroportFactoryExecuteMsg),
    ProvideLiquidity {
        assets: [AssetUnchecked; 2],
        slippage_tolerance: Option<Decimal>,
        auto_stake: Option<bool>,
    },
    WithdrawLiquidity {
        pair_address: String,
        amount: Option<Uint128>,
        recipient: Option<Recipient>,
    },
    StakeLp {
        lp_token_contract: String,
        amount: Option<Uint128>,
    },
    UnstakeLp {
        lp_token_contract: String,
        amount: Option<Uint128>,
    },
    ClaimLpStakingRewards {
        lp_token_contract: String,
        auto_stake: Option<bool>,
    },
    StakeAstro {
        amount: Option<Uint128>,
    },
    UnstakeAstro {
        amount: Option<Uint128>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
}

#[cw_serde]
pub enum Cw20HookMsg {
    Swapper(SwapperCw20HookMsg),
}
