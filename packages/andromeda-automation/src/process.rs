use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct ProcessComponent {
    pub name: String,
    pub ado_type: String,
    pub instantiate_msg: Binary,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub process: Vec<ProcessComponent>,
    pub name: String,
    pub primitive_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    AddProcessComponent { component: ProcessComponent },
    ClaimOwnership { name: Option<String> },
    Fire { msg: Binary },
    ProxyMessage { name: String, msg: Binary },
    UpdateAddress { name: String, addr: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(String)]
    GetAddress { name: String },
    #[returns(Vec<ProcessComponent>)]
    GetComponents {},
    #[returns(bool)]
    ComponentExists { name: String },
    #[returns(Vec<ComponentAddress>)]
    GetAddresses {},
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub name: String,
}

#[cw_serde]
pub struct ComponentAddress {
    pub name: String,
    pub address: String,
}
