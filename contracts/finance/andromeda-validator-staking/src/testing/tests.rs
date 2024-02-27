use crate::{
    contract::instantiate,
    testing::mock_querier::{mock_dependencies_custom, VALID_VALIDATOR},
};

use andromeda_std::{error::ContractError, testing::mock_querier::MOCK_KERNEL_CONTRACT};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    Addr, DepsMut, Response,
};

use andromeda_finance::validator_staking::InstantiateMsg;

const OWNER: &str = "creator";

fn init(deps: DepsMut, default_validator: Addr) -> Result<Response, ContractError> {
    let msg = InstantiateMsg {
        default_validator,
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        modules: None,
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg)
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom();

    let fake_validator = Addr::unchecked("fake_validator");
    let res = init(deps.as_mut(), fake_validator);
    assert_eq!(ContractError::InvalidValidator {}, res.unwrap_err());

    let default_validator = Addr::unchecked(VALID_VALIDATOR);
    let res = init(deps.as_mut(), default_validator).unwrap();
    assert_eq!(0, res.messages.len());
}
