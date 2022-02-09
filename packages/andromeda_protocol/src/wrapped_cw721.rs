use crate::communication::modules::Module;
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
    //TODO : Replace with Fetch contract once that is in.
    pub factory_contract: String,
    /// The cw721 contract can be instantiated or an existing address can be used. In the case that
    /// an existing address is used, the minter must be set to be this contract.
    pub cw721_instantiate_type: InstantiateType,
    /// Whether or not the cw721 token can be unwrapped once it is wrapped.
    pub can_unwrap: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
