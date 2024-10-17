use andromeda_data_storage::point::{PointCoordinate, PointRestriction};
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const DATA: Item<PointCoordinate> = Item::new("data");
pub const DATA_OWNER: Item<Addr> = Item::new("data_owner");
pub const RESTRICTION: Item<PointRestriction> = Item::new("restriction");
