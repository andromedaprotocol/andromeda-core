use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Create new token
    Create {
        name: String,
        symbol: String,
    },
    UpdateCodeId {
        code_id_key: String,
        code_id: u64,
    },
    /// Update token contract address by symbol
    UpdateAddress {
        symbol: String,
        new_address: String,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    /// Query token contract address by its symbol
    #[returns(AddressResponse)]
    GetAddress { symbol: String },
    /// All code IDs for Andromeda contracts
    #[returns(u64)]
    CodeId { key: String },
}

#[cw_serde]
pub struct AddressResponse {
    pub address: String,
}
