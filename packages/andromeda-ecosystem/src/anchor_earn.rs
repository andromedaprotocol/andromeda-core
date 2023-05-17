use common::ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    pub primitive_contract: String,
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
}

#[cw_serde]
pub struct PositionResponse {
    pub recipient: Recipient,
    pub aust_amount: Uint128,
}
