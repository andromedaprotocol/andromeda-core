use crate::swapper::{SwapperCw20HookMsg, SwapperMsg};
use astroport::{
    factory::ExecuteMsg as AstroportFactoryExecuteMsg,
    router::{Cw20HookMsg as AstroportRouterCw20HookMsg, ExecuteMsg as AstroportRouterExecuteMsg},
    staking::{
        Cw20HookMsg as AstroportStakingCw20HookMsg, ExecuteMsg as AstroportStakingExecuteMsg,
    },
};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub astroport_factory_contract: String,
    pub astroport_router_contract: String,
    pub astroport_staking_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Swapper(SwapperMsg),
    Receive(Cw20ReceiveMsg),
    AstroportFactoryExecuteMsg(AstroportFactoryExecuteMsg),
    AstroportRouterExecuteMsg(AstroportRouterExecuteMsg),
    AstroportStakingExecuteMsg(AstroportStakingExecuteMsg),
    UpdateOwner {
        address: String,
    },
    UpdateConfig {
        astroport_factory_contract: Option<String>,
        astroport_router_contract: Option<String>,
        astroport_staking_contract: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractOwner {},
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Swapper(SwapperCw20HookMsg),
    AstroportRouterCw20HookMsg(AstroportRouterCw20HookMsg),
    AstroportStakingCw20HookMsg(AstroportStakingCw20HookMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub astroport_factory_contract: String,
    pub astroport_router_contract: String,
    pub astroport_staking_contract: String,
}
