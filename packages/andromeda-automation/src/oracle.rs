use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    // The contract which we'll query data from
    pub target_address: AndrAddress,
    // The query message's binary
    pub message_binary: String,
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
