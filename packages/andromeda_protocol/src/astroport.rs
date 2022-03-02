use crate::{
    communication::{AndromedaMsg, AndromedaQuery, Recipient},
    swapper::{SwapperCw20HookMsg, SwapperMsg},
};
use astroport::{asset::Asset, factory::ExecuteMsg as AstroportFactoryExecuteMsg};
use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub astroport_factory_contract: String,
    pub astroport_router_contract: String,
    pub astroport_staking_contract: String,
    pub astro_token_contract: String,
    pub xastro_token_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    Swapper(SwapperMsg),
    Receive(Cw20ReceiveMsg),
    AstroportFactoryExecuteMsg(AstroportFactoryExecuteMsg),
    UpdateConfig {
        astroport_factory_contract: Option<String>,
        astroport_router_contract: Option<String>,
        astroport_staking_contract: Option<String>,
        astro_token_contract: Option<String>,
        xastro_token_contract: Option<String>,
    },
    ProvideLiquidity {
        assets: [Asset; 2],
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

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Swapper(SwapperCw20HookMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub astroport_factory_contract: String,
    pub astroport_router_contract: String,
    pub astroport_staking_contract: String,
    pub astro_token_contract: String,
    pub xastro_token_contract: String,
}
