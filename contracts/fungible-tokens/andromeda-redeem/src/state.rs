use andromeda_fungible_tokens::redeem::RedemptionCondition;
use cw_storage_plus::Item;

pub const REDEMPTION_CONDITION: Item<RedemptionCondition> = Item::new("redemption_condition");
