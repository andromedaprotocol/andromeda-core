use andromeda_non_fungible_tokens::cw721_offers::Offer;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

pub const CW721_CONTRACT: Item<String> = Item::new("cw721_contract");

pub struct OfferIndexes<'a> {
    /// (purchaser, token_id))
    pub purchaser: MultiIndex<'a, String, Offer, String>,
}

impl<'a> IndexList<Offer> for OfferIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Offer>> + '_> {
        let v: Vec<&dyn Index<Offer>> = vec![&self.purchaser];
        Box::new(v.into_iter())
    }
}

pub fn offers<'a>() -> IndexedMap<'a, &'a str, Offer, OfferIndexes<'a>> {
    let indexes = OfferIndexes {
        purchaser: MultiIndex::new(|e| e.purchaser.clone(), "ownership", "offer_purchaser"),
    };
    IndexedMap::new("ownership", indexes)
}
