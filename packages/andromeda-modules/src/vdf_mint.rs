use andromeda_std::amp::AndrAddr;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint64};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub cw721_address: AndrAddr,
    pub actors: Option<Vec<AndrAddr>>,
    pub mint_cooldown_minutes: Option<Uint64>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    #[attrs(restricted, nonpayable)]
    AddActors { actors: Vec<AndrAddr> },
    #[attrs(restricted, nonpayable)]
    RemoveActors { actors: Vec<AndrAddr> },
    #[attrs(nonpayable)]
    VdfMint { token_id: String, owner: AndrAddr },
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetActorsResponse)]
    GetActors {},
    #[returns(GetMintCooldownMinutesResponse)]
    GetMintCooldownMinutes {},
    #[returns(GetLastMintTimestampSecondsResponse)]
    GetLastMintTimestampSeconds {},
}

#[cw_serde]
pub struct GetActorsResponse {
    pub actors: Vec<Addr>,
}

#[cw_serde]
pub struct GetMintCooldownMinutesResponse {
    pub mint_cooldown_minutes: Uint64,
}

#[cw_serde]
pub struct GetLastMintTimestampSecondsResponse {
    pub last_mint_timestamp_seconds: Uint64,
}
