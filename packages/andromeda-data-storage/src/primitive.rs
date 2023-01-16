use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    primitive::Primitive,
};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// If key is not specified the default key will be used.
    SetValue {
        key: Option<String>,
        value: Primitive,
    },
    /// If key is not specified the default key will be used.
    DeleteValue {
        key: Option<String>,
    },
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
}
