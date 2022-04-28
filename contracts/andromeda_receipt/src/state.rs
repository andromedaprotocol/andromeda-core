use andromeda_protocol::{
    ownership::is_contract_owner,
    receipt::{Config, Receipt},
};
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
    Ok(is_contract_owner(storage, addr.to_string())?
        || addr.eq(&config.minter)
        || config.moderators.contains(addr))
}

// increase receipt ID
pub fn increment_num_receipt(storage: &mut dyn Storage) -> StdResult<Uint128> {
    let mut receipt_count = NUM_RECEIPT.load(storage).unwrap_or_default();
    //Changed type conversion from explicit to implicit. [AKP-01] (Delete when reviewed)
    //Added checked_add function to make sure that no overflow occurs [ACP-02] (Delete when reviewed)
    let res = receipt_count.checked_add(Uint128::from(1u128));
    //Check that no overflow, else panic. 
    let _res = match res {
        Err(error) => panic!("Problem adding: {:?}", error),
        _ => {receipt_count = res.unwrap()}
    };
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
    use andromeda_protocol::ownership::CONTRACT_OWNER;
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
        };
        let mut deps = mock_dependencies(&[]);

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &owner.to_string())
            .unwrap();
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
