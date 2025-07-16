use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

pub const MINT_RECIPIENT_AMOUNT: Item<(String, Uint128)> = Item::new("mint_recipient_amount");

pub const OSMOSIS_MSG_CREATE_DENOM_ID: u64 = 1;
pub const OSMOSIS_MSG_BURN_ID: u64 = 2;
