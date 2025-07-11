use andromeda_std::amp::AndrAddr;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

#[cw_serde]
pub struct FactoryInfo {
    pub factory_contract: AndrAddr,
    pub amount: Uint128,
    pub user: Addr,
}

pub const LOCKED_TOKENS: Item<FactoryInfo> = Item::new("locked_tokens");
