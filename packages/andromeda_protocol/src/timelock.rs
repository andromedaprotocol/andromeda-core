use cosmwasm_std::{Coin, StdResult, Storage};
use cw721::Expiration;
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const HELD_FUNDS: Map<String, Escrow> = Map::new("funds");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    HoldFunds { expire: Expiration },
    ReleaseFunds {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetLockedFunds { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Escrow {
    pub coins: Vec<Coin>,
    pub expire: Expiration,
}

pub fn hold_funds(funds: Escrow, storage: &mut dyn Storage, addr: String) -> StdResult<()> {
    HELD_FUNDS.save(storage, addr.clone(), &funds)
}

pub fn release_funds(storage: &mut dyn Storage, addr: String) {
    HELD_FUNDS.remove(storage, addr.clone());
}

pub fn get_funds(storage: &dyn Storage, addr: String) -> StdResult<Option<Escrow>> {
    HELD_FUNDS.may_load(storage, addr)
}
