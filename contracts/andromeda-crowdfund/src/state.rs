use cw_storage_plus::{Item, Map};
use cosmwasm_std::{Uint128, Coin};
use cw0::Expiration;

pub const CONFIG: Item<Config> = Item::new("config");

/// Sale started if and only if STATE.may_load is Some and !duration.is_expired()
pub const STATE: <Item<State>> = Item::new("state");

/// The amount of funds to send to recipient if sale successful. This already
/// takes into account the royalties and taxes.
pub const AMOUNT_TO_SEND: Item<Uint128> = Item::new("amount_to_send");

/// Key is address of buyer
pub const BUYERS: Map<&str, Vec<Purchase>>  = Map::new("buyers");

///  on purchase, add remaining_amount to AMOUNT_TO_SEND
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct Purchase {
	token_id: String,
	// amount of tax paid
	denom: String,
	tax_amount: Uint128,
	// sub messages for rates sending
	msgs: Vec<SubMsg>
}

/// Can be updated if sale NOT ongoing.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct Config {
	token_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct State {
	duration: Expiration,
	price: Coin,
	min: Uint128,
}
