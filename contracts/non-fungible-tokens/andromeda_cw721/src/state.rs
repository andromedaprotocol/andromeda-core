use common::mission::AndrAddress;
use cw_storage_plus::Item;

// Key must not be "minter" as that is reserved by cw721_base contract.
pub const ANDR_MINTER: Item<AndrAddress> = Item::new("andr_minter");
