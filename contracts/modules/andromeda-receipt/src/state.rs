use ado_base::state::ADOContract;
use andromeda_modules::receipt::{Config, Receipt};
use common::error::ContractError;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

pub const CONFIG: Item<Config> = Item::new("config");
const RECEIPT: Map<u128, Receipt> = Map::new("receipt");
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
    match res {
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
    RECEIPT.save(storage, receipt_id.u128(), receipt)
}
pub fn read_receipt(storage: &dyn Storage, receipt_id: Uint128) -> StdResult<Receipt> {
    RECEIPT.load(storage, receipt_id.u128())
}

#[cfg(test)]
mod tests {
    const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;
    use common::ado_base::InstantiateMsg as BaseInstantiateMsg;

    #[test]
    fn test_can_mint() {
        let minter = String::from("minter");
        let operator = String::from("operator");
        let owner = String::from("owner");
        let anyone = String::from("anyone");

        let config = Config {
            minter: minter.clone(),
        };
        let mut deps = mock_dependencies();
        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                mock_info(&owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "receipt".to_string(),
                    ado_version: CONTRACT_VERSION.to_string(),
                    operators: None,
                    modules: None,
                    primitive_contract: None,
                    kernel_address: None,
                },
            )
            .unwrap();
        let info = mock_info(&owner, &[]);

        ADOContract::default()
            .execute_update_operators(deps.as_mut(), info, vec![operator.clone()])
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
