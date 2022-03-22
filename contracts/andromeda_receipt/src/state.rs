use ado_base::state::ADOContract;
use andromeda_protocol::receipt::{Config, Receipt};
use common::error::ContractError;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map, U128Key};

pub const CONFIG: Item<Config> = Item::new("config");
const RECEIPT: Map<U128Key, Receipt> = Map::new("receipt");
const NUM_RECEIPT: Item<Uint128> = Item::new("num_receipt");

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn can_mint_receipt(storage: &dyn Storage, addr: &str) -> Result<bool, ContractError> {
    let config = CONFIG.load(storage)?;
    Ok(ADOContract::default().is_owner_or_operator(storage, addr)? || addr.eq(&config.minter))
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
        _ => receipt_count = res.unwrap(),
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
    use cosmwasm_std::{testing::mock_dependencies, Addr};

    use super::*;

    #[test]
    fn test_can_mint() {
        let minter = String::from("minter");
        let operator = String::from("operator");
        let owner = String::from("owner");
        let anyone = String::from("anyone");

        let config = Config {
            minter: minter.clone(),
        };
        let mut deps = mock_dependencies(&[]);
        ADOContract::default()
            .operators
            .save(deps.as_mut().storage, &operator, &true)
            .unwrap();

        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(owner.to_string()))
            .unwrap();
        CONFIG.save(deps.as_mut().storage, &config).unwrap();

        let anyone_resp = can_mint_receipt(deps.as_ref().storage, &anyone).unwrap();
        assert!(!anyone_resp);

        let owner_resp = can_mint_receipt(deps.as_ref().storage, &owner).unwrap();
        assert!(owner_resp);

        let minter_resp = can_mint_receipt(deps.as_ref().storage, &minter).unwrap();
        assert!(minter_resp);

        let operator_resp = can_mint_receipt(deps.as_ref().storage, &operator).unwrap();
        assert!(operator_resp);
    }
}
