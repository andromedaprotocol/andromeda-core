use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MissionComponent {
    pub name: String,
    pub ado_type: String,
    pub instantiate_msg: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub operators: Vec<String>,
    pub mission: Vec<MissionComponent>,
    pub xfer_ado_ownership: bool,
    pub name: String,
    pub primitive_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    AddMissionComponent { component: MissionComponent },
    ClaimOwnership { name: Option<String> },
    ProxyMessage { name: String, msg: Binary },
    UpdateAddress { name: String, addr: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    GetAddress { name: String },
    GetComponents {},
    GetAddresses {},
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    // use super::*;
}
