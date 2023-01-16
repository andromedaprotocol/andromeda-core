use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {
    // The contract which we'll query data from
    pub target_address: String,

    // The query message's binary
    pub message_binary: Binary,

    // The query's expected return type (u64, bool ...) or (CountResponse, PriceResponse ...)
    pub return_type: TypeOfResponse,

    // Specific element in the custom return struct
    pub response_element: Option<String>,

    pub kernel_address: Option<String>,
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
pub enum RegularTypes {
    String,
    Bool,
    Uint128,
}

// Response type we expect from our query. Support for additional response types will be added down the line
#[cw_serde]
pub enum CustomTypes {
    CounterResponse,
}

#[cw_serde]
pub enum TypeOfResponse {
    RegularType(RegularTypes),
    CustomType(CustomTypes),
}
