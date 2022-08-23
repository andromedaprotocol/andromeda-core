use common::ado_base::{modules::Module, AndromedaMsg, AndromedaQuery};
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstantiateType {
    New(Cw721Specification),
    Address(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Cw721Specification {
    pub name: String,
    pub symbol: String,
    pub modules: Option<Vec<Module>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub primitive_contract: String,
    /// The cw721 contract can be instantiated or an existing address can be used. In the case that
    /// an existing address is used, the minter must be set to be this contract.
    pub cw721_instantiate_type: InstantiateType,
    /// Whether or not the cw721 token can be unwrapped once it is wrapped.
    pub can_unwrap: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    AndrReceive(AndromedaMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw721HookMsg {
    Wrap { wrapped_token_id: Option<String> },
    Unwrap {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
