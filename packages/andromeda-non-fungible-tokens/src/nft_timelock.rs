use common::ado_base::{AndromedaMsg, AndromedaQuery};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Transfers the given token to the recipient once the time lock has expired.
    Claim {
        lock_id: String,
    },
    Lock {
        recipient: Option<String>,
        nft_id: String,
        lock_time: u64,
        andromeda_cw721_contract: String,
    },
    UpdateOwner {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    LockedToken { lock_id: String },
    Owner {},
}
