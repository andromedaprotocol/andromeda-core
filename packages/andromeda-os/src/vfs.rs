use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the Kernel contract on chain
    pub kernel_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    // Receives an AMP Packet for relaying
    // AMPReceive(AMPPkt),
    AddPath {
        name: String,
        address: Addr,
    },
    RegisterUser {
        username: String,
        address: Option<Addr>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    ResolvePath { path: String },
}
