use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

// The condition ADO we want to send our bool to
pub const COUNT: Item<Uint128> = Item::new("count");
