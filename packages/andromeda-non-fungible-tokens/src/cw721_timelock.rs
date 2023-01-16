use common::ado_base::{AndromedaMsg, AndromedaQuery};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cw721::Cw721ReceiveMsg;
use cw_utils::Expiration;

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Transfers the given token to the recipient once the time lock has expired.
    Claim {
        lock_id: String,
    },
    ReceiveNft(Cw721ReceiveMsg),
}

#[cw_serde]
pub enum Cw721HookMsg {
    /// Locks the token in the contract for the desired time while setting the recipient as the sender if not provided.
    StartLock {
        recipient: Option<String>,
        lock_time: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(LockDetails)]
    LockedToken { lock_id: String },
}

#[cw_serde]
pub struct LockDetails {
    pub recipient: String,
    pub expiration: Expiration,
    pub nft_id: String,
    pub nft_contract: String,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
