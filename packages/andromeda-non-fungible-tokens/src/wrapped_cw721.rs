use andromeda_os::messages::AMPPkt;
use common::ado_base::{modules::Module, AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cw721::Cw721ReceiveMsg;

#[cw_serde]
pub enum InstantiateType {
    New(Cw721Specification),
    Address(String),
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct Cw721Specification {
    pub name: String,
    pub symbol: String,
    pub modules: Option<Vec<Module>>,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub primitive_contract: String,
    /// The cw721 contract can be instantiated or an existing address can be used. In the case that
    /// an existing address is used, the minter must be set to be this contract.
    pub cw721_instantiate_type: InstantiateType,
    /// Whether or not the cw721 token can be unwrapped once it is wrapped.
    pub can_unwrap: bool,
    pub kernel_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    AMPReceive(AMPPkt),
    ReceiveNft(Cw721ReceiveMsg),
}

#[cw_serde]
pub enum Cw721HookMsg {
    Wrap { wrapped_token_id: Option<String> },
    Unwrap {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(String)]
    NFTContractAddress {},
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
