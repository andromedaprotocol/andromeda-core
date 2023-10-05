use andromeda_non_fungible_tokens::marketplace::{SaleStateResponse, Status};
use common::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Order, Storage, SubMsg, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};

const MAX_LIMIT: u64 = 30;
const DEFAULT_LIMIT: u64 = 10;

#[cw_serde]
pub struct TokenSaleState {
    pub coin_denom: String,
    pub sale_id: Uint128,
    pub owner: String,
    pub token_id: String,
    pub token_address: String,
    pub price: Uint128,
    pub status: Status,
}

#[cw_serde]
pub struct Purchase {
    /// The token id being purchased.
    pub token_id: String,
    /// Amount of tax paid.
    pub tax_amount: Uint128,
    /// sub messages for sending funds for rates.
    pub msgs: Vec<SubMsg>,
    /// The purchaser of the token.
    pub purchaser: String,
}

#[cw_serde]
#[derive(Default)]
pub struct SaleInfo {
    pub sale_ids: Vec<Uint128>,
    pub token_address: String,
    pub token_id: String,
}

impl SaleInfo {
    pub fn last(&self) -> Option<&Uint128> {
        self.sale_ids.last()
    }

    pub fn push(&mut self, e: Uint128) {
        self.sale_ids.push(e)
    }
}

impl From<TokenSaleState> for SaleStateResponse {
    fn from(token_sale_state: TokenSaleState) -> SaleStateResponse {
        SaleStateResponse {
            coin_denom: token_sale_state.coin_denom,
            sale_id: token_sale_state.sale_id,
            status: token_sale_state.status,
            price: token_sale_state.price,
        }
    }
}

pub const NEXT_SALE_ID: Item<Uint128> = Item::new("next_sale_id");

pub const TOKEN_SALE_STATE: Map<u128, TokenSaleState> = Map::new("sale_token_state");

pub struct SaleIdIndices<'a> {
    /// PK: token_id + token_address
    /// Secondary key: token_address
    pub token: MultiIndex<'a, String, SaleInfo, String>,
}

impl<'a> IndexList<SaleInfo> for SaleIdIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<SaleInfo>> + '_> {
        let v: Vec<&dyn Index<SaleInfo>> = vec![&self.token];
        Box::new(v.into_iter())
    }
}

pub fn sale_infos<'a>() -> IndexedMap<'a, &'a str, SaleInfo, SaleIdIndices<'a>> {
    let indexes = SaleIdIndices {
        token: MultiIndex::new(
            |_pk: &[u8], r| r.token_address.clone(),
            "ownership",
            "token_index",
        ),
    };
    IndexedMap::new("ownership", indexes)
}

pub fn read_sale_infos(
    storage: &dyn Storage,
    token_address: String,
    start_after: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<SaleInfo>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let keys: Vec<String> = sale_infos()
        .idx
        .token
        .prefix(token_address)
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<Result<Vec<String>, _>>()?;

    let mut res: Vec<SaleInfo> = vec![];
    for key in keys.iter() {
        res.push(sale_infos().load(storage, key)?);
    }
    Ok(res)
}
