use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{Decimal, OwnedDeps, Validator};

pub const VALID_VALIDATOR: &str = "valid_validator";

pub fn mock_dependencies_custom() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let valid_validator = Validator {
        address: String::from(VALID_VALIDATOR),
        commission: Decimal::percent(1),
        max_commission: Decimal::percent(3),
        max_change_rate: Decimal::percent(1),
    };

    let mut custom_querier: MockQuerier = MockQuerier::default();
    custom_querier.update_staking("uandr", &[valid_validator], &[]);
    let storage = MockStorage::default();
    OwnedDeps {
        storage,
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: std::marker::PhantomData,
    }
}
