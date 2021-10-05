use cosmwasm_std::{Coin, StdResult, Storage};
use cw721::Expiration;
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::modules::address_list::AddressListModule;

pub const HELD_FUNDS: Map<String, Escrow> = Map::new("funds");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub address_list: Option<AddressListModule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    HoldFunds {
        expiration: Expiration,
        recipient: Option<String>,
    },
    ReleaseFunds {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetLockedFunds { address: String },
    GetTimelockConfig {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetLockedFundsResponse {
    pub funds: Option<Escrow>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetTimelockConfigResponse {
    pub address_list: Option<AddressListModule>,
    pub address_list_contract: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Escrow {
    pub coins: Vec<Coin>,
    pub expiration: Expiration,
    pub recipient: String,
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
