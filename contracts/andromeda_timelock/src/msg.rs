use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw721::{ Expiration };

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    HoldFunds {
        expire: Expiration
    },
    ReleaseFunds {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetLockedFunds {
        address: String
    },
}
