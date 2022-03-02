use crate::{
    communication::{encode_binary, query_get, AndromedaMsg, AndromedaQuery},
    error::ContractError,
};
use cosmwasm_std::{Addr, Binary, Coin, QuerierWrapper, StdError, Storage, Uint128};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Used to store the addresses of each ADO within the mission
pub const ADO_ADDRESSES: Map<String, Addr> = Map::new("ado_addresses");
pub const ADO_DESCRIPTORS: Map<i64, MissionComponent> = Map::new("ado_descriptors");

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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
}

#[cfg(test)]
mod tests {
    use super::*;
}
