use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateCodeId { code_id_key: String, code_id: u64 },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// All code IDs for Andromeda contracts
    #[returns(u64)]
    CodeId { key: String },
    #[returns(Option<String>)]
    ADOType { code_id: u64 },
}
