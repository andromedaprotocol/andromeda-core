use andromeda_fungible_tokens::cw20_redeem::RedemptionCondition;
use andromeda_std::amp::AndrAddr;
use cw_storage_plus::Item;

pub const TOKEN_ADDRESS: Item<AndrAddr> = Item::new("token_address");
pub const REDEMPTION_CONDITION: Item<RedemptionCondition> = Item::new("redemption_condition");
