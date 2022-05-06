use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, DepsMut, Response, Uint128,
};
use cw_storage_plus::U64Key;

use crate::{
    contract::{execute, instantiate, query},
    state::{batches, Batch, Config, CONFIG, NEXT_ID},
};

use andromeda_finance::vesting::{ExecuteMsg, InstantiateMsg, QueryMsg};
use common::{ado_base::recipient::Recipient, error::ContractError, withdraw::WithdrawalType};

fn init(deps: DepsMut) -> Response {
    let msg = InstantiateMsg {
        recipient: Recipient::Addr("recipient".to_string()),
        is_multi_batch_enabled: true,
        denom: "uusd".to_string(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);

    let res = init(deps.as_mut());

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "vesting"),
        res
    );

    assert_eq!(
        Config {
            recipient: Recipient::Addr("recipient".to_string()),
            is_multi_batch_enabled: true,
            denom: "uusd".to_string()
        },
        CONFIG.load(deps.as_ref().storage).unwrap()
    );
}

#[test]
fn test_create_batch_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 1,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_create_batch_no_funds() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 1,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Creating a batch must be accompanied with a single native fund".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_create_batch_invalid_denom() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(500, "uluna"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 1,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Invalid denom".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_create_batch_valid_denom_zero_amount() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(0, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 1,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(
        ContractError::InvalidFunds {
            msg: "Funds must be non-zero".to_string()
        },
        res.unwrap_err()
    );
}

#[test]
fn test_create_batch_release_unit_zero() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 0,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::InvalidZeroAmount {}, res.unwrap_err());
}

#[test]
fn test_create_batch_release_amount_zero() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::InvalidZeroAmount {}, res.unwrap_err());
}

#[test]
fn test_create_batch() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let current_time = mock_env().block.time.seconds();

    assert_eq!(
        Response::new()
            .add_attribute("action", "create_batch")
            .add_attribute("amount", "100")
            .add_attribute("lockup_end", current_time.to_string())
            .add_attribute("release_unit", "10")
            .add_attribute("release_amount", "Amount(Uint128(10))"),
        res
    );

    let batch = batches()
        .load(deps.as_ref().storage, U64Key::new(1))
        .unwrap();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::zero(),
            lockup_end: current_time,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time,
        },
        batch
    );

    assert_eq!(2, NEXT_ID.load(deps.as_ref().storage).unwrap());

    // Try to create another batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: Some(100),
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "create_batch")
            .add_attribute("amount", "100")
            .add_attribute("lockup_end", (current_time + 100).to_string())
            .add_attribute("release_unit", "10")
            .add_attribute("release_amount", "Amount(Uint128(10))"),
        res
    );

    let batch = batches()
        .load(deps.as_ref().storage, U64Key::new(2))
        .unwrap();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::zero(),
            lockup_end: current_time + 100,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time + 100,
        },
        batch
    );

    assert_eq!(3, NEXT_ID.load(deps.as_ref().storage).unwrap());
}

#[test]
fn test_create_batch_multi_batch_not_supported() {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        recipient: Recipient::Addr("recipient".to_string()),
        is_multi_batch_enabled: false,
        denom: "uusd".to_string(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info("owner", &coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: Some(100),
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    let current_time = mock_env().block.time.seconds();

    assert_eq!(
        Response::new()
            .add_attribute("action", "create_batch")
            .add_attribute("amount", "100")
            .add_attribute("lockup_end", (current_time + 100).to_string())
            .add_attribute("release_unit", "10")
            .add_attribute("release_amount", "Amount(Uint128(10))"),
        res
    );

    let batch = batches()
        .load(deps.as_ref().storage, U64Key::new(1))
        .unwrap();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::zero(),
            lockup_end: current_time + 100,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: current_time + 100,
        },
        batch
    );

    assert_eq!(2, NEXT_ID.load(deps.as_ref().storage).unwrap());

    // Try to create another batch.
    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::MultiBatchNotSupported {}, res.unwrap_err());
}

#[test]
fn test_claim_batch_unauthorized() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_claim_batch_still_locked() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: Some(100),
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Claim batch.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::FundsAreLocked {}, res.unwrap_err());
}

#[test]
fn test_claim_batch_no_funds_available() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Claim batch.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // This is because, the first payment becomes available after 10 seconds.
    assert_eq!(ContractError::WithdrawalIsEmpty {}, res.unwrap_err());
}

#[test]
fn test_claim_batch_single_claim() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Claim batch.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

    let mut env = mock_env();
    // A single release is available.
    env.block.time = env.block.time.plus_seconds(release_unit);

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                amount: coins(10, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "10")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "90"),
        res
    );
    let lockup_end = mock_env().block.time.seconds();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(10),
            lockup_end,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end + release_unit,
        },
        batches().load(deps.as_ref().storage, 1u64.into()).unwrap()
    );
}

#[test]
fn test_claim_batch_middle_of_interval() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Claim batch.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

    let mut env = mock_env();
    // Only halfway to first release.
    env.block.time = env.block.time.plus_seconds(release_unit / 2);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());

    assert_eq!(ContractError::WithdrawalIsEmpty {}, res.unwrap_err());

    // First release available and halfway to second -> result is rounding down.
    env.block.time = env.block.time.plus_seconds(release_unit);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                amount: coins(10, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "10")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "90"),
        res
    );
    let lockup_end = mock_env().block.time.seconds();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(10),
            lockup_end,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end + release_unit,
        },
        batches().load(deps.as_ref().storage, 1u64.into()).unwrap()
    );
}

#[test]
fn test_claim_batch_multiple_claims() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mut env = mock_env();

    // 4 releases are available.
    env.block.time = env.block.time.plus_seconds(4 * release_unit);

    // Claim only the first release.
    let msg = ExecuteMsg::Claim {
        number_of_claims: Some(1),
        batch_id: 1,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                amount: coins(10, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "10")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "90"),
        res
    );
    let lockup_end = mock_env().block.time.seconds();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(10),
            lockup_end,
            release_unit,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end + release_unit,
        },
        batches().load(deps.as_ref().storage, 1u64.into()).unwrap()
    );

    // Claim the rest of the releases.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                amount: coins(30, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "30")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "60"),
        res
    );
    let lockup_end = mock_env().block.time.seconds();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(40),
            lockup_end,
            release_unit,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end + 4 * release_unit,
        },
        batches().load(deps.as_ref().storage, 1u64.into()).unwrap()
    );
}

#[test]
fn test_claim_batch_all_releases() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mut env = mock_env();

    // All releases are available and then some (10 * release_unit would be when all releases
    // become available).
    env.block.time = env.block.time.plus_seconds(15 * release_unit);

    // Claim only the first release.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                amount: coins(100, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "100")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "0"),
        res
    );
    let lockup_end = mock_env().block.time.seconds();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(100),
            lockup_end,
            release_unit,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end + 15 * release_unit,
        },
        batches().load(deps.as_ref().storage, 1u64.into()).unwrap()
    );

    // Try to claim again.
    let res = execute(deps.as_mut(), env.clone(), info, msg);

    assert_eq!(ContractError::WithdrawalIsEmpty {}, res.unwrap_err());
}

#[test]
fn test_claim_batch_too_high_of_claim() {
    let mut deps = mock_dependencies(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        stake: false,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mut env = mock_env();
    // A single release is available.
    env.block.time = env.block.time.plus_seconds(release_unit);

    // Try to claim 3 releases.
    let msg = ExecuteMsg::Claim {
        number_of_claims: Some(3),
        batch_id: 1,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                // Only one gets claimed
                amount: coins(10, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "10")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "90"),
        res
    );
    let lockup_end = mock_env().block.time.seconds();

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(10),
            lockup_end,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end + release_unit,
        },
        batches().load(deps.as_ref().storage, 1u64.into()).unwrap()
    );
}
