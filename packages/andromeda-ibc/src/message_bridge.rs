use common::ado_base::AndromedaQuery;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    SendMessage {
        chain: String,
        recipient: String,
        message: Binary,
    },
    SaveChannel {
        channel: String,
        chain: String,
    },
}

#[cw_serde]
pub enum IbcExecuteMsg {
    SendMessage { recipient: String, message: Binary },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(String)]
    ChannelID { chain: String },
    #[returns(Vec<String>)]
    SupportedChains {},
}

#[cw_serde]
pub enum Chain {
    Juno,
}
