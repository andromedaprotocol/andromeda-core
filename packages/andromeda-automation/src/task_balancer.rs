use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub max: u64,
    // Code IDS are u64
    pub storage_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    Add { contract: String },
    UpdateAdmin { new_admin: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    GetSize {},
}

#[cw_serde]
pub enum LoopQueryMsg {
    GetSize {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetSizeResponse {
    pub size: Uint128,
}

#[cw_serde]
pub enum StorageExecuteMsg {
    Store { contract: String },
}

#[cw_serde]
pub struct StorageInstantiateMsg {
    pub contract: String,
}
