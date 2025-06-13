use crate::{
    contract::{execute, instantiate},
    testing::mock_querier::{mock_dependencies_custom, DEFAULT_VALIDATOR, VALID_VALIDATOR},
};

use andromeda_std::{error::ContractError, testing::mock_querier::MOCK_KERNEL_CONTRACT};
use cosmwasm_std::{
    coin,
    testing::{message_info, mock_env},
    Addr, Response, StakingMsg,
};

use andromeda_finance::validator_staking::{ExecuteMsg, InstantiateMsg};

const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";

fn init(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        crate::testing::mock_querier::WasmMockQuerier,
    >,
    default_validator: Addr,
) -> Result<Response, ContractError> {
    let msg = InstantiateMsg {
        default_validator,
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg)
}

#[test]
fn test_instantiate() {
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        crate::testing::mock_querier::WasmMockQuerier,
    > = mock_dependencies_custom(&[]);

    let fake_validator = deps.api.addr_make("fake_validator");
    let res = init(&mut deps, fake_validator);
    assert_eq!(ContractError::InvalidValidator {}, res.unwrap_err());

    let default_validator = Addr::unchecked(DEFAULT_VALIDATOR);
    let res = init(&mut deps, default_validator).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_stake_with_invalid_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    let default_validator = Addr::unchecked(DEFAULT_VALIDATOR);
    init(&mut deps, default_validator).unwrap();

    let msg = ExecuteMsg::Stake { validator: None };

    let info = message_info(
        &Addr::unchecked(OWNER),
        &[coin(100, "uandr"), coin(100, "usdc")],
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    assert_eq!(res, ContractError::ExceedsMaxAllowedCoins {});
}

#[test]
fn test_stake_with_default_validator() {
    let mut deps = mock_dependencies_custom(&[]);
    let default_validator = Addr::unchecked(DEFAULT_VALIDATOR);
    init(&mut deps, default_validator).unwrap();

    let msg = ExecuteMsg::Stake { validator: None };

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uandr")]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    let expected_res: Response = Response::new()
        .add_message(StakingMsg::Delegate {
            validator: DEFAULT_VALIDATOR.to_string(),
            amount: coin(100, "uandr"),
        })
        .add_attribute("action", "validator-stake")
        .add_attribute("from", OWNER.to_string())
        .add_attribute("to", DEFAULT_VALIDATOR.to_string())
        .add_attribute("amount", "100".to_string());

    assert_eq!(res.unwrap(), expected_res);
}

#[test]
fn test_stake_with_validator() {
    let mut deps = mock_dependencies_custom(&[]);
    let default_validator = Addr::unchecked(DEFAULT_VALIDATOR);
    let valid_validator = Addr::unchecked(VALID_VALIDATOR);
    init(&mut deps, default_validator).unwrap();

    let msg = ExecuteMsg::Stake {
        validator: Some(valid_validator),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uandr")]);

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    let expected_res: Response = Response::new()
        .add_message(StakingMsg::Delegate {
            validator: VALID_VALIDATOR.to_string(),
            amount: coin(100, "uandr"),
        })
        .add_attribute("action", "validator-stake")
        .add_attribute("from", OWNER.to_string())
        .add_attribute("to", VALID_VALIDATOR.to_string())
        .add_attribute("amount", "100".to_string());

    assert_eq!(res.unwrap(), expected_res);
}

#[test]
fn test_stake_with_invalid_validator() {
    let mut deps = mock_dependencies_custom(&[]);
    let fake_validator = Addr::unchecked("fake_validator");
    let default_validator = Addr::unchecked(DEFAULT_VALIDATOR);
    init(&mut deps, default_validator).unwrap();

    let msg = ExecuteMsg::Stake {
        validator: Some(fake_validator),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uandr")]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    assert_eq!(res, ContractError::InvalidValidator {});
}

#[test]
fn test_unauthorized_unstake() {
    let mut deps = mock_dependencies_custom(&[]);
    let default_validator = Addr::unchecked(DEFAULT_VALIDATOR);
    let valid_validator = Addr::unchecked(VALID_VALIDATOR);
    init(&mut deps, default_validator).unwrap();

    let msg = ExecuteMsg::Stake {
        validator: Some(valid_validator.clone()),
    };

    let info = message_info(&Addr::unchecked(OWNER), &[coin(100, "uandr")]);

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Unstake {
        validator: Some(valid_validator),
        amount: None,
    };

    let info = message_info(&Addr::unchecked("other"), &[coin(100, "uandr")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}
