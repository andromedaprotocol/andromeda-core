use andromeda_finance::timelock::Escrow;
use andromeda_std::error::ContractError;
use cosmwasm_std::{Order, Storage};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};

const DEFAULT_LIMIT: u32 = 10u32;
const MAX_LIMIT: u32 = 30u32;

pub struct EscrowIndexes<'a> {
    /// (recipient, encoded(vec![owner, recipient]))
    pub owner: MultiIndex<'a, String, Escrow, Vec<u8>>,
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
            |_pk: &[u8], e| e.recipient_addr.clone(),
            "ownership",
            "escrow_owner",
        ),
    };
    IndexedMap::new("ownership", indexes)
}

pub fn get_keys_for_recipient(
    storage: &dyn Storage,
    recipient_addr: &str,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<Vec<u8>>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let keys: Result<Vec<Vec<u8>>, ContractError> = escrows()
        .idx
        .owner
        .prefix(recipient_addr.to_string())
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|k| Ok(k?))
        .collect();
    keys
}

pub fn get_key(owner: &str, recipient: &str) -> Vec<u8> {
    [owner.as_bytes(), recipient.as_bytes()].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_key() {
        let owner = "owner";
        let recipient = "recipient";
        // Want to ensure the keys are different.
        assert_ne!(get_key(owner, recipient), get_key(recipient, owner));
    }
}
