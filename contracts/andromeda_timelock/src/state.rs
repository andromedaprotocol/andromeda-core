use andromeda_protocol::{modules::address_list::AddressListModule, timelock::Escrow};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const STATE: Item<State> = Item::new("state");

pub const TEST: Map<(&str, &str), Escrow> = Map::new("test");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub address_list: Option<AddressListModule>,
}

pub struct EscrowIndexes<'a> {
    pub owner: MultiIndex<'a, (String, Vec<u8>), Escrow>,
}

impl<'a> IndexList<Escrow> for EscrowIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Escrow>> + '_> {
        let v: Vec<&dyn Index<Escrow>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn escrows<'a>() -> IndexedMap<'a, Vec<u8>, Escrow, EscrowIndexes<'a>> {
    let indexes = EscrowIndexes {
        owner: MultiIndex::new(
            |e, k| (e.recipient.get_addr().to_string(), k.into()),
            "ownership",
            "escrow_owner",
        ),
    };
    IndexedMap::new("ownership", indexes)
}
