use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage, StdResult};
use cw_storage_plus::{Item,Map};
pub const CONTRACT_OWNER: Item<String> = Item::new("contractowner");
pub const ADDRESSES : Map<String, String> = Map::new("addresses");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: Addr,
}

pub const STATE: Item<State> = Item::new("state");
pub fn store_address(storage: &mut dyn Storage, name: String, contract_address: &String) -> StdResult<()> {
    ADDRESSES.save(storage, name, &contract_address)
}

pub fn read_address(storage: &dyn Storage, name: String) -> StdResult<String> {
    ADDRESSES.load(storage, name)
}

pub fn is_contract_owner(storage: &dyn Storage, addr: String) -> StdResult<bool> {
    let owner = CONTRACT_OWNER.load(storage)?;
    Ok(addr.eq(&owner))
}
