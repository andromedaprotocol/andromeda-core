use andromeda_std::{
    amp::Recipient, common::withdraw::WithdrawalType, error::ContractError,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    coin, coins, from_json,
    testing::{mock_env, mock_info, MockQuerier, MOCK_CONTRACT_ADDR},
    Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, DistributionMsg, FullDelegation, GovMsg,
    Response, StakingMsg, Uint128, Validator, VoteOption,
};
use cw_utils::Duration;

use crate::{
    contract::{execute, instantiate, query},
    state::{batches, Batch, CONFIG, NEXT_ID},
    testing::mock_querier::mock_dependencies_custom,
};

use andromeda_finance::vesting::{BatchResponse, Config, ExecuteMsg, InstantiateMsg, QueryMsg};

const DEFAULT_VALIDATOR: &str = "validator";
const UNBONDING_BLOCK_DURATION: u64 = 5;

fn init(deps: DepsMut) -> Response {
    let msg = InstantiateMsg {
        recipient: Recipient::from_string("recipient"),
        is_multi_batch_enabled: true,
        denom: "uusd".to_string(),
        unbonding_duration: Duration::Height(UNBONDING_BLOCK_DURATION),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        modules: None,
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

fn sample_validator(addr: &str) -> Validator {
    Validator {
        address: addr.into(),
        commission: Decimal::percent(3),
        max_commission: Decimal::percent(10),
        max_change_rate: Decimal::percent(1),
    }
}

fn sample_delegation(addr: &str, amount: Coin) -> FullDelegation {
    let can_redelegate = amount.clone();
    let accumulated_rewards = coins(0, &amount.denom);
    FullDelegation {
        validator: addr.into(),
        delegator: Addr::unchecked(MOCK_CONTRACT_ADDR),
        amount,
        can_redelegate,
        accumulated_rewards,
    }
}

fn set_delegation(querier: &mut MockQuerier, amount: u128, denom: &str) {
    querier.update_staking(
        "ustake",
        &[sample_validator(DEFAULT_VALIDATOR)],
        &[sample_delegation(DEFAULT_VALIDATOR, coin(amount, denom))],
    )
}

fn create_batch(
    deps: DepsMut,
    lockup_duration: Option<u64>,
    release_unit: u64,
    release_amount: WithdrawalType,
) -> Response {
    // Create batch with half of the release_unit.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration,
        release_unit,
        release_amount,
        validator_to_delegate_to: None,
    };

    let info = mock_info("owner", &coins(100, "uusd"));
    execute(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);

    let res = init(deps.as_mut());

    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("type", "vesting")
            .add_attribute("kernel_address", MOCK_KERNEL_CONTRACT)
            .add_attribute("owner", "owner"),
        res
    );

    assert_eq!(
        Config {
            recipient: Recipient::from_string("recipient"),
            is_multi_batch_enabled: true,
            denom: "uusd".to_string(),
            unbonding_duration: Duration::Height(UNBONDING_BLOCK_DURATION)
        },
        CONFIG.load(deps.as_ref().storage).unwrap()
    );
}

#[test]
fn test_create_batch_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 1,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        validator_to_delegate_to: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_create_batch_no_funds() {
    let mut deps = mock_dependencies_custom(&[]);

    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 1,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        validator_to_delegate_to: None,
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
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(500, "uluna"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 1,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        validator_to_delegate_to: None,
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
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(0, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 1,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        validator_to_delegate_to: None,
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
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 0,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        validator_to_delegate_to: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::InvalidZeroAmount {}, res.unwrap_err());
}

#[test]
fn test_create_batch_release_amount_zero() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::zero()),
        validator_to_delegate_to: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::InvalidZeroAmount {}, res.unwrap_err());
}

#[test]
fn test_create_batch() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
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

    let batch = batches().load(deps.as_ref().storage, 1).unwrap();

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
        validator_to_delegate_to: None,
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

    let batch = batches().load(deps.as_ref().storage, 2).unwrap();

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
fn test_create_batch_and_delegate() {
    let mut deps = mock_dependencies_custom(&[coin(1000, "uusd")]);
    init(deps.as_mut());

    let info = mock_info("owner", &coins(100, "uusd"));

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: Some(DEFAULT_VALIDATOR.to_owned()),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let current_time = mock_env().block.time.seconds();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::SetWithdrawAddress {
                    address: "owner".to_string()
                }
            ))
            .add_message(CosmosMsg::Staking(StakingMsg::Delegate {
                validator: DEFAULT_VALIDATOR.to_string(),
                amount: coin(100, "uusd")
            }))
            .add_attribute("action", "create_batch")
            .add_attribute("amount", "100")
            .add_attribute("lockup_end", current_time.to_string())
            .add_attribute("release_unit", "10")
            .add_attribute("release_amount", "Amount(Uint128(10))")
            .add_attribute("action", "delegate")
            .add_attribute("validator", DEFAULT_VALIDATOR)
            .add_attribute("amount", "100"),
        res
    );
}

#[test]
fn test_create_batch_multi_batch_not_supported() {
    let mut deps = mock_dependencies_custom(&[]);
    let msg = InstantiateMsg {
        recipient: Recipient::from_string("recipient"),
        is_multi_batch_enabled: false,
        denom: "uusd".to_string(),
        unbonding_duration: Duration::Height(0u64),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        modules: None,
    };

    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info("owner", &coins(100, "uusd"));

    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: Some(100),
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
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

    let batch = batches().load(deps.as_ref().storage, 1).unwrap();

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
    let mut deps = mock_dependencies_custom(&[]);
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
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: Some(100),
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
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
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
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
fn test_claim_batch_all_funds_delegated() {
    let mut deps = mock_dependencies_custom(&[coin(1000, "uusd")]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

    // Delegate tokens
    let msg = ExecuteMsg::Delegate {
        amount: None,
        validator: DEFAULT_VALIDATOR.to_owned(),
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Skip time to first release.
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(10);

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
fn test_claim_batch_some_funds_delegated() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit: 10,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

    // Delegate tokens
    let msg = ExecuteMsg::Delegate {
        amount: Some(Uint128::new(70)),
        validator: DEFAULT_VALIDATOR.to_owned(),
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Skip time to where all funds available.
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(1000);

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(30, "uusd"));

    // Claim batch.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                // Only 30 are available
                amount: coins(30, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "30")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "70"),
        res
    );
}

#[test]
fn test_claim_batch_single_claim() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

    // Skip time.
    let mut env = mock_env();
    // A single release is available.
    env.block.time = env.block.time.plus_seconds(release_unit);

    // Query created batch.
    let msg = QueryMsg::Batch { id: 1 };
    let res: BatchResponse = from_json(query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();

    let lockup_end = mock_env().block.time.seconds();
    assert_eq!(
        BatchResponse {
            id: 1,
            amount: Uint128::new(100),
            amount_claimed: Uint128::zero(),
            amount_available_to_claim: Uint128::new(10),
            number_of_available_claims: Uint128::new(1),
            lockup_end,
            release_unit,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end,
        },
        res
    );

    // Claim batch.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

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
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );
}

#[test]
fn test_claim_batch_not_nice_numbers_single_release() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(7, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(7, "uusd"));

    // Skip time.
    let mut env = mock_env();
    // A single release is available.
    env.block.time = env.block.time.plus_seconds(release_unit);

    // Claim batch.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                amount: coins(7, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "7")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "0"),
        res
    );
    let lockup_end = mock_env().block.time.seconds();

    assert_eq!(
        Batch {
            amount: Uint128::new(7),
            amount_claimed: Uint128::new(7),
            lockup_end,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end + release_unit,
        },
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );
}

#[test]
fn test_claim_batch_not_nice_numbers_multiple_releases() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(14, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(14, "uusd"));

    // Skip time.
    let mut env = mock_env();
    // Two releases are out.
    env.block.time = env.block.time.plus_seconds(2 * release_unit);

    // Claim batch.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                amount: coins(14, "uusd")
            })
            .add_attribute("action", "claim")
            .add_attribute("amount", "14")
            .add_attribute("batch_id", "1")
            .add_attribute("amount_left", "0"),
        res
    );
    let lockup_end = mock_env().block.time.seconds();

    assert_eq!(
        Batch {
            amount: Uint128::new(14),
            amount_claimed: Uint128::new(14),
            lockup_end,
            release_unit: 10,
            release_amount: WithdrawalType::Amount(Uint128::new(10)),
            last_claimed_release_time: lockup_end + 2 * release_unit,
        },
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );
}

#[test]
fn test_claim_batch_middle_of_interval() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

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
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );
}

#[test]
fn test_claim_batch_multiple_claims() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

    let mut env = mock_env();

    // 4 releases are available.
    env.block.time = env.block.time.plus_seconds(4 * release_unit);

    // Claim only the first release.
    let msg = ExecuteMsg::Claim {
        number_of_claims: Some(1),
        batch_id: 1,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );

    // Claim the rest of the releases.
    let msg = ExecuteMsg::Claim {
        number_of_claims: None,
        batch_id: 1,
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

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
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );
}

#[test]
fn test_claim_batch_all_releases() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

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
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );

    // Try to claim again.
    let res = execute(deps.as_mut(), env, info, msg);

    assert_eq!(ContractError::WithdrawalIsEmpty {}, res.unwrap_err());
}

#[test]
fn test_claim_batch_too_high_of_claim() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let info = mock_info("owner", &coins(100, "uusd"));

    let release_unit = 10;

    // Create batch.
    let msg = ExecuteMsg::CreateBatch {
        lockup_duration: None,
        release_unit,
        release_amount: WithdrawalType::Amount(Uint128::new(10)),
        validator_to_delegate_to: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

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
                // Only one gets claim
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
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );
}

#[test]
fn test_claim_all_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::ClaimAll {
        up_to_time: None,
        limit: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_claim_all() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let release_unit = 10;

    let release_amount = WithdrawalType::Amount(Uint128::new(10));
    // Create batch.
    create_batch(deps.as_mut(), None, release_unit, release_amount.clone());

    // Create batch with half of the release_unit.
    create_batch(
        deps.as_mut(),
        None,
        release_unit / 2,
        release_amount.clone(),
    );

    // Create batch with a different release_unit scale (not a factor).
    create_batch(deps.as_mut(), None, 12, release_amount.clone());

    // Create batch that is still locked up.
    create_batch(
        deps.as_mut(),
        Some(100),
        release_unit,
        release_amount.clone(),
    );

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(400, "uusd"));

    // Speed up time.
    let mut env = mock_env();
    env.block.time = env.block.time.plus_seconds(release_unit * 2);

    // Query batches
    let msg = QueryMsg::Batches {
        start_after: None,
        limit: None,
    };
    let res: Vec<BatchResponse> =
        from_json(query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();

    let lockup_end = mock_env().block.time.seconds();
    assert_eq!(
        vec![
            BatchResponse {
                id: 1,
                amount: Uint128::new(100),
                amount_claimed: Uint128::zero(),
                amount_available_to_claim: Uint128::new(20),
                number_of_available_claims: Uint128::new(2),
                lockup_end,
                release_unit,
                release_amount: WithdrawalType::Amount(Uint128::new(10)),
                last_claimed_release_time: lockup_end,
            },
            BatchResponse {
                id: 2,
                amount: Uint128::new(100),
                amount_claimed: Uint128::zero(),
                amount_available_to_claim: Uint128::new(40),
                number_of_available_claims: Uint128::new(4),
                lockup_end,
                release_unit: release_unit / 2,
                release_amount: WithdrawalType::Amount(Uint128::new(10)),
                last_claimed_release_time: lockup_end,
            },
            BatchResponse {
                id: 3,
                amount: Uint128::new(100),
                amount_claimed: Uint128::zero(),
                amount_available_to_claim: Uint128::new(10),
                number_of_available_claims: Uint128::new(1),
                lockup_end,
                release_unit: 12,
                release_amount: WithdrawalType::Amount(Uint128::new(10)),
                last_claimed_release_time: lockup_end,
            },
            BatchResponse {
                id: 4,
                amount: Uint128::new(100),
                amount_claimed: Uint128::zero(),
                amount_available_to_claim: Uint128::zero(),
                number_of_available_claims: Uint128::zero(),
                lockup_end: lockup_end + 100,
                release_unit,
                release_amount: WithdrawalType::Amount(Uint128::new(10)),
                last_claimed_release_time: lockup_end + 100,
            },
        ],
        res
    );

    // Claim all
    let msg = ExecuteMsg::ClaimAll {
        up_to_time: None,
        limit: None,
    };

    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(BankMsg::Send {
                to_address: "recipient".to_string(),
                // 20 from the first, 40 from the second, 10 from the third.
                amount: coins(20 + 40 + 10, "uusd")
            })
            .add_attribute("action", "claim_all")
            .add_attribute("last_batch_id_processed", "3"),
        res
    );

    let lockup_end = mock_env().block.time.seconds();
    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(20),
            lockup_end,
            release_unit,
            release_amount: release_amount.clone(),
            last_claimed_release_time: lockup_end + release_unit * 2,
        },
        batches().load(deps.as_ref().storage, 1u64).unwrap()
    );

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(40),
            lockup_end,
            release_unit: release_unit / 2,
            release_amount: release_amount.clone(),
            last_claimed_release_time: lockup_end + release_unit * 2,
        },
        batches().load(deps.as_ref().storage, 2u64).unwrap()
    );

    assert_eq!(
        Batch {
            amount: Uint128::new(100),
            amount_claimed: Uint128::new(10),
            lockup_end,
            release_unit: 12,
            release_amount,
            last_claimed_release_time: lockup_end + 12,
        },
        batches().load(deps.as_ref().storage, 3u64).unwrap()
    );
}

#[test]
fn test_delegate_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::Delegate {
        amount: None,
        validator: DEFAULT_VALIDATOR.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_delegate_no_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::Delegate {
        amount: None,
        validator: DEFAULT_VALIDATOR.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::InvalidZeroAmount {}, res.unwrap_err());
}

#[test]
fn test_delegate() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::Delegate {
        amount: None,
        validator: DEFAULT_VALIDATOR.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::SetWithdrawAddress {
                    address: "owner".to_string()
                }
            ))
            .add_message(CosmosMsg::Staking(StakingMsg::Delegate {
                validator: DEFAULT_VALIDATOR.to_string(),
                amount: coin(100, "uusd")
            }))
            .add_attribute("action", "delegate")
            .add_attribute("validator", DEFAULT_VALIDATOR)
            .add_attribute("amount", "100"),
        res
    );
}

#[test]
fn test_delegate_more_than_balance() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    deps.querier
        .base
        .update_balance(MOCK_CONTRACT_ADDR, coins(100, "uusd"));

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::Delegate {
        amount: Some(Uint128::new(200)),
        validator: DEFAULT_VALIDATOR.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::SetWithdrawAddress {
                    address: "owner".to_string()
                }
            ))
            .add_message(CosmosMsg::Staking(StakingMsg::Delegate {
                validator: DEFAULT_VALIDATOR.to_string(),
                amount: coin(100, "uusd")
            }))
            .add_attribute("action", "delegate")
            .add_attribute("validator", DEFAULT_VALIDATOR)
            .add_attribute("amount", "100"),
        res
    );
}

#[test]
fn test_redelegate_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::Redelegate {
        amount: None,
        from: DEFAULT_VALIDATOR.to_string(),
        to: "other_validator".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_redelegate_no_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::Redelegate {
        amount: None,
        from: DEFAULT_VALIDATOR.to_string(),
        to: "other_validator".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::InvalidZeroAmount {}, res.unwrap_err());
}

#[test]
fn test_redelegate() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    set_delegation(&mut deps.querier.base, 100, "uusd");

    let msg = ExecuteMsg::Redelegate {
        amount: None,
        from: DEFAULT_VALIDATOR.to_string(),
        to: "other_validator".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::SetWithdrawAddress {
                    address: "owner".to_string()
                }
            ))
            .add_message(CosmosMsg::Staking(StakingMsg::Redelegate {
                src_validator: DEFAULT_VALIDATOR.to_owned(),
                dst_validator: "other_validator".to_string(),
                amount: coin(100, "uusd")
            }))
            .add_attribute("action", "redelegate")
            .add_attribute("from", DEFAULT_VALIDATOR)
            .add_attribute("to", "other_validator")
            .add_attribute("amount", "100"),
        res
    );
}

#[test]
fn test_redelegate_more_than_max() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    set_delegation(&mut deps.querier.base, 100, "uusd");

    let msg = ExecuteMsg::Redelegate {
        amount: Some(Uint128::new(200)),
        from: DEFAULT_VALIDATOR.to_string(),
        to: "other_validator".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::SetWithdrawAddress {
                    address: "owner".to_string()
                }
            ))
            .add_message(CosmosMsg::Staking(StakingMsg::Redelegate {
                src_validator: DEFAULT_VALIDATOR.to_owned(),
                dst_validator: "other_validator".to_string(),
                amount: coin(100, "uusd")
            }))
            .add_attribute("action", "redelegate")
            .add_attribute("from", DEFAULT_VALIDATOR)
            .add_attribute("to", "other_validator")
            .add_attribute("amount", "100"),
        res
    );
}

#[test]
fn test_undelegate_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::Undelegate {
        amount: None,
        validator: DEFAULT_VALIDATOR.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_undelegate_no_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::Undelegate {
        amount: None,
        validator: DEFAULT_VALIDATOR.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::InvalidZeroAmount {}, res.unwrap_err());
}

#[test]
fn test_undelegate() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    set_delegation(&mut deps.querier.base, 100, "uusd");

    let msg = ExecuteMsg::Undelegate {
        amount: None,
        validator: DEFAULT_VALIDATOR.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::SetWithdrawAddress {
                    address: "owner".to_string()
                }
            ))
            .add_message(CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: DEFAULT_VALIDATOR.to_owned(),
                amount: coin(100, "uusd")
            }))
            .add_attribute("action", "undelegate")
            .add_attribute("validator", DEFAULT_VALIDATOR)
            .add_attribute("amount", "100"),
        res
    );
}

#[test]
fn test_undelegate_more_than_max() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    set_delegation(&mut deps.querier.base, 100, "uusd");

    let msg = ExecuteMsg::Undelegate {
        amount: Some(Uint128::new(200)),
        validator: DEFAULT_VALIDATOR.to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::SetWithdrawAddress {
                    address: "owner".to_string()
                }
            ))
            .add_message(CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: DEFAULT_VALIDATOR.to_owned(),
                amount: coin(100, "uusd")
            }))
            .add_attribute("action", "undelegate")
            .add_attribute("validator", DEFAULT_VALIDATOR)
            .add_attribute("amount", "100"),
        res
    );
}

#[test]
fn test_withdraw_rewards_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::WithdrawRewards {};

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_vote_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("not_owner", &[]);

    let msg = ExecuteMsg::Vote {
        proposal_id: 1,
        vote: VoteOption::Yes,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
}

#[test]
fn test_withdraw_rewards() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::WithdrawRewards {};

    deps.querier.base.update_staking(
        "ustake",
        &[
            sample_validator("validator1"),
            sample_validator("validator2"),
            sample_validator("validator3"),
        ],
        &[
            sample_delegation("validator1", coin(100, "ustake")),
            sample_delegation("validator2", coin(100, "ustake")),
            sample_delegation("validator3", coin(100, "ustake")),
        ],
    );

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_attribute("action", "withdraw_rewards")
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::SetWithdrawAddress {
                    address: "owner".to_string()
                }
            ))
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::WithdrawDelegatorReward {
                    validator: "validator1".to_string()
                }
            ))
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::WithdrawDelegatorReward {
                    validator: "validator2".to_string()
                }
            ))
            .add_message(CosmosMsg::Distribution(
                DistributionMsg::WithdrawDelegatorReward {
                    validator: "validator3".to_string()
                }
            )),
        res
    );
}

#[test]
fn test_vote() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());

    let info = mock_info("owner", &[]);

    let msg = ExecuteMsg::Vote {
        proposal_id: 1,
        vote: VoteOption::Yes,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        Response::new()
            .add_message(CosmosMsg::Gov(GovMsg::Vote {
                proposal_id: 1,
                vote: VoteOption::Yes
            }))
            .add_attribute("action", "vote")
            .add_attribute("proposal_id", "1")
            .add_attribute("vote", "Yes"),
        res
    );
}
