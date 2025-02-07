use andromeda_finance::mint_burn::OrderInfo;
use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

pub const NEXT_ORDER_ID: Item<Uint128> = Item::new("next_order_id");
pub const ORDERS: Map<u128, OrderInfo> = Map::new("orders");
