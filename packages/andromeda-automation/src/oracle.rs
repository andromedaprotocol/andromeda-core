use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    // The contract which we'll query data from
    pub target_address: String,

    // The query message's binary
    pub message_binary: String,

    // The query's expected return type
    pub expected_type: Types,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),

    #[returns(String)]
    CurrentTarget {},

    #[returns(String)]
    Target {},

    #[returns(String)]
    StoredMessage {},
}

// Type of response we expect from our query
#[cw_serde]
pub enum Types {
    String,
    Bool,
    Uint128,
}
