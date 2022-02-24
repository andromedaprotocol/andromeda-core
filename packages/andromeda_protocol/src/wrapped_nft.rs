use crate::modules::ModuleDefinition;
use cosmwasm_std::Addr;
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub andromeda_factory_addr: Addr,
    pub name: String,
    pub symbol: String,
    pub modules: Vec<ModuleDefinition>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
}
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    TokenInfo { token_id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw721HookMsg {
    Wrap {},
    Unwrap {},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ConfigResponse {
    pub name: String,
    pub symbol: String,
    pub factory_addr: String,
    pub token_addr: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct TokenInfoResponse {
    pub original_token_id: String,
    pub original_token_addr: String,
}
