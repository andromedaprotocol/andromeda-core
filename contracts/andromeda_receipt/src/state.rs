use andromeda_protocol::receipt::{Config, Receipt};
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map, U128Key};

pub const CONFIG: Item<Config> = Item::new("config");
const RECEIPT: Map<U128Key, Receipt> = Map::new("receipt");
const NUM_RECEIPT: Item<Uint128> = Item::new("num_receipt");

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn can_mint_receipt(storage: &dyn Storage, addr: &String) -> StdResult<bool> {
    let config = CONFIG.load(storage)?;
    Ok(addr.eq(&config.owner) || addr.eq(&config.minter) || config.moderators.contains(addr))
}

// increase receipt ID
pub fn increment_num_receipt(storage: &mut dyn Storage) -> StdResult<Uint128> {
    let mut receipt_count = NUM_RECEIPT.load(storage).unwrap_or_default();
    receipt_count = receipt_count + Uint128::from(1 as u128);
    NUM_RECEIPT.save(storage, &receipt_count)?;
    Ok(receipt_count)
}

pub fn store_receipt(
    storage: &mut dyn Storage,
    receipt_id: Uint128,
    receipt: &Receipt,
) -> StdResult<()> {
    RECEIPT.save(storage, U128Key::from(receipt_id.u128()), receipt)
}
pub fn read_receipt(storage: &dyn Storage, receipt_id: Uint128) -> StdResult<Receipt> {
    RECEIPT.load(storage, U128Key::from(receipt_id.u128()))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_can_mint() {
        let minter = String::from("minter");
        let moderator = String::from("moderator");
        let owner = String::from("owner");
        let anyone = String::from("anyone");

        let config = Config {
            minter: minter.clone(),
            moderators: vec![moderator.clone()],
            owner: owner.clone(),
        };
        let mut deps = mock_dependencies(&[]);

        CONFIG.save(deps.as_mut().storage, &config).unwrap();

        let anyone_resp = can_mint_receipt(deps.as_ref().storage, &anyone).unwrap();
        assert!(!anyone_resp);

        let owner_resp = can_mint_receipt(deps.as_ref().storage, &owner).unwrap();
        assert!(owner_resp);

        let minter_resp = can_mint_receipt(deps.as_ref().storage, &minter).unwrap();
        assert!(minter_resp);

        let moderator_resp = can_mint_receipt(deps.as_ref().storage, &moderator).unwrap();
        assert!(moderator_resp);
    }
}
