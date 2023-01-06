use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    SendMessage {
        channel: String,
        target: String,
        message: Binary,
    },
}

#[cw_serde]
pub enum IbcExecuteMsg {
    SendMessage { target: String, message: Binary },
}

#[cw_serde]
pub enum QueryMsg {}
