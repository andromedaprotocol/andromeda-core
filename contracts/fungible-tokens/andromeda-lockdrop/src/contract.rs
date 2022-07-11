// The mars lockdrop contract was used as a base for this.
// https://github.com/mars-protocol/mars-periphery/tree/main/contracts/lockdrop

use cosmwasm_std::{
    entry_point, from_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_asset::Asset;

use ado_base::ADOContract;
use andromeda_fungible_tokens::lockdrop::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StateResponse,
    UserInfoResponse,
};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError, require,
};

use crate::state::{Config, State, CONFIG, STATE, USER_INFO};

// version info for migration info
const CONTRACT_NAME: &str = "andromeda-lockup";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

//----------------------------------------------------------------------------------------
// Entry Points
//----------------------------------------------------------------------------------------

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // CHECK :: init_timestamp needs to be valid
    require(
        msg.init_timestamp >= env.block.time.seconds(),
        ContractError::StartTimeInThePast {
            current_seconds: env.block.time.seconds(),
            current_block: env.block.height,
        },
    )?;

    // CHECK :: deposit_window,withdrawal_window need to be valid (withdrawal_window < deposit_window)
    require(
        msg.deposit_window > 0
            && msg.withdrawal_window > 0
            && msg.withdrawal_window < msg.deposit_window,
        ContractError::InvalidWindow {},
    )?;

    let config = Config {
        bootstrap_contract_address: msg.bootstrap_contract,
        init_timestamp: msg.init_timestamp,
        deposit_window: msg.deposit_window,
        withdrawal_window: msg.withdrawal_window,
        lockdrop_incentives: Uint128::zero(),
        incentive_token: msg.incentive_token,
        native_denom: msg.native_denom,
    };

    CONFIG.save(deps.storage, &config)?;
    STATE.save(deps.storage, &State::default())?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "lockdrop".to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::DepositNative {} => execute_deposit_native(deps, env, info),
        ExecuteMsg::WithdrawNative { amount } => execute_withdraw_native(deps, env, info, amount),
        ExecuteMsg::DepositToBootstrap { amount } => {
            execute_deposit_to_bootstrap(deps, env, info, amount)
        }
        ExecuteMsg::EnableClaims {} => execute_enable_claims(deps, env, info),
        ExecuteMsg::ClaimRewards {} => execute_claim_rewards(deps, env, info),
        ExecuteMsg::WithdrawProceeds { recipient } => {
            execute_withdraw_proceeds(deps, env, info, recipient)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // CHECK :: Tokens sent > 0
    require(
        !cw20_msg.amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Number of tokens should be > 0".to_string(),
        },
    )?;

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::IncreaseIncentives {} => {
            execute_increase_incentives(deps, env, info, cw20_msg.amount)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
        QueryMsg::State {} => encode_binary(&query_state(deps)?),
        QueryMsg::UserInfo { address } => encode_binary(&query_user_info(deps, env, address)?),
        QueryMsg::WithdrawalPercentAllowed { timestamp } => {
            encode_binary(&query_max_withdrawable_percent(deps, env, timestamp)?)
        }
    }
}

//----------------------------------------------------------------------------------------
// Execute Functions
//----------------------------------------------------------------------------------------

/// @dev Facilitates increasing token incentives that are to be distributed as Lockdrop participation reward
/// @params amount : Number of tokens which are to be added to current incentives
pub fn execute_increase_incentives(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    require(
        info.sender == config.incentive_token,
        ContractError::InvalidFunds {
            msg: "Only incentive tokens are valid".to_string(),
        },
    )?;

    require(
        is_withdraw_open(env.block.time.seconds(), &config),
        ContractError::TokenAlreadyBeingDistributed {},
    )?;

    config.lockdrop_incentives += amount;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "incentives_increased")
        .add_attribute("amount", amount))
}

/// @dev Facilitates NATIVE deposits.
pub fn execute_deposit_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let depositor_address = info.sender;

    // CHECK :: Lockdrop deposit window open
    require(
        is_deposit_open(env.block.time.seconds(), &config),
        ContractError::DepositWindowClosed {},
    )?;

    // Check if multiple native coins sent by the user
    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Must deposit a single fund".to_string(),
        },
    )?;

    let native_token = info.funds.first().unwrap();
    require(
        native_token.denom == config.native_denom,
        ContractError::InvalidFunds {
            msg: format!("Only {} accepted", config.native_denom),
        },
    )?;

    // CHECK ::: Amount needs to be valid
    require(
        !native_token.amount.is_zero(),
        ContractError::InvalidFunds {
            msg: "Amount must be greater than 0".to_string(),
        },
    )?;

    // USER INFO :: RETRIEVE --> UPDATE
    let mut user_info = USER_INFO
        .may_load(deps.storage, &depositor_address)?
        .unwrap_or_default();

    user_info.total_native_locked += native_token.amount;

    // STATE :: UPDATE --> SAVE
    state.total_native_locked += native_token.amount;

    STATE.save(deps.storage, &state)?;
    USER_INFO.save(deps.storage, &depositor_address, &user_info)?;

    Ok(Response::new()
        .add_attribute("action", "lock_native")
        .add_attribute("user", depositor_address)
        .add_attribute("ust_deposited", native_token.amount))
}

/// @dev Facilitates NATIVE withdrawal from an existing Lockup position. Can only be called when deposit / withdrawal window is open
/// @param withdraw_amount : NATIVE amount to be withdrawn
pub fn execute_withdraw_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    withdraw_amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let mut user_info = USER_INFO.load(deps.storage, &info.sender)?;

    // USER ADDRESS AND LOCKUP DETAILS
    let withdrawer_address = info.sender;

    // CHECK :: Lockdrop withdrawal window open
    require(
        is_withdraw_open(env.block.time.seconds(), &config),
        ContractError::InvalidWithdrawal {
            msg: Some("Withdrawals not available".to_string()),
        },
    )?;

    // Check :: Amount should be within the allowed withdrawal limit bounds
    let max_withdrawal_percent = allowed_withdrawal_percent(env.block.time.seconds(), &config);
    let max_withdrawal_allowed = user_info.total_native_locked * max_withdrawal_percent;
    let withdraw_amount = withdraw_amount.unwrap_or(max_withdrawal_allowed);
    require(
        withdraw_amount <= max_withdrawal_allowed,
        ContractError::InvalidWithdrawal {
            msg: Some(format!(
                "Amount exceeds max allowed withdrawal limit of {}",
                max_withdrawal_allowed
            )),
        },
    )?;

    // Update withdrawal flag after the deposit window
    if env.block.time.seconds() > config.init_timestamp + config.deposit_window {
        // CHECK :: Max 1 withdrawal allowed
        require(
            !user_info.withdrawal_flag,
            ContractError::InvalidWithdrawal {
                msg: Some("Max 1 withdrawal allowed".to_string()),
            },
        )?;

        user_info.withdrawal_flag = true;
    }

    user_info.total_native_locked -= withdraw_amount;

    USER_INFO.save(deps.storage, &withdrawer_address, &user_info)?;

    // STATE :: UPDATE --> SAVE
    state.total_native_locked -= withdraw_amount;
    STATE.save(deps.storage, &state)?;

    // COSMOS_MSG ::TRANSFER WITHDRAWN native token
    let native_token = Asset::native(config.native_denom, withdraw_amount);
    let withdraw_msg = native_token.transfer_msg(withdrawer_address.clone())?;

    Ok(Response::new()
        .add_message(withdraw_msg)
        .add_attribute("action", "withdraw_native")
        .add_attribute("user", withdrawer_address)
        .add_attribute("amount", withdraw_amount))
}

/// Function callable only by Bootstrap contract (if it is specified) to enable TOKEN Claims by users.
/// Called along-with Bootstrap contract's LP Pool provide liquidity tx. If it is not
/// specified then anyone can execute this when the phase has ended.
pub fn execute_enable_claims(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    // If bootstrap is specified then only it can enable claims.
    if let Some(bootstrap_contract_address) = &config.bootstrap_contract_address {
        let app_contract = contract.get_app_contract(deps.storage)?;
        let bootstrap_contract_address =
            bootstrap_contract_address.get_address(deps.api, &deps.querier, app_contract)?;

        // CHECK :: ONLY BOOTSTRAP CONTRACT CAN CALL THIS FUNCTION
        require(
            info.sender == bootstrap_contract_address,
            ContractError::Unauthorized {},
        )?;
    }

    // CHECK :: Claims can only be enabled after the deposit / withdrawal windows are closed
    require(
        !is_withdraw_open(env.block.time.seconds(), &config),
        ContractError::PhaseOngoing {},
    )?;

    // CHECK ::: Claims are only enabled once
    require(
        !state.are_claims_allowed,
        ContractError::ClaimsAlreadyAllowed {},
    )?;
    state.are_claims_allowed = true;

    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", "enable_claims"))
}

/// @dev Function to delegate part of the token rewards to be used for LP Bootstrapping via
/// bootstrap
/// @param amount : Number of tokens to delegate
pub fn execute_deposit_to_bootstrap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let user_address = info.sender;

    // CHECK :: Have the deposit / withdraw windows concluded
    require(
        !is_withdraw_open(env.block.time.seconds(), &config),
        ContractError::PhaseOngoing {},
    )?;

    // CHECK :: Can users withdraw their tokens ? -> if so, then delegation is no longer allowed
    require(
        !state.are_claims_allowed,
        ContractError::ClaimsAlreadyAllowed {},
    )?;

    // CHECK :: Bootstrap contract address should be set
    require(
        config.bootstrap_contract_address.is_some(),
        ContractError::NoSavedBootstrapContract {},
    )?;

    let mut user_info = USER_INFO
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    let total_incentives = config
        .lockdrop_incentives
        .multiply_ratio(user_info.total_native_locked, state.total_native_locked);

    // CHECK :: token to delegate cannot exceed user's unclaimed token balance
    let available_amount = total_incentives - user_info.delegated_incentives;
    require(
        amount <= available_amount,
        ContractError::InvalidFunds {
            msg: format!(
                "Amount cannot exceed user's unclaimed token balance. Tokens to delegate = {}, Max delegatable tokens = {}",
                amount,
                available_amount
            ),
        },
    )?;

    // UPDATE STATE
    user_info.delegated_incentives += amount;
    state.total_delegated += amount;

    // SAVE UPDATED STATE
    STATE.save(deps.storage, &state)?;
    USER_INFO.save(deps.storage, &user_address, &user_info)?;

    // COSMOS_MSG ::Delegate tokens to the LP Bootstrapping via Bootstrap contract
    // TODO: When Bootstrapping contract is created add this message.

    Ok(Response::new()
        .add_attribute("action", "deposit_to_bootstrap")
        .add_attribute("user_address", user_address)
        .add_attribute("delegated_amount", amount))
}

/// @dev Function to claim Rewards from lockdrop.
pub fn execute_claim_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    let user_address = info.sender;
    let mut user_info = USER_INFO
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    require(
        !user_info.lockdrop_claimed,
        ContractError::LockdropAlreadyClaimed {},
    )?;
    require(
        !user_info.total_native_locked.is_zero(),
        ContractError::NoLockup {},
    )?;
    require(state.are_claims_allowed, ContractError::ClaimsNotAllowed {})?;

    let total_incentives = config
        .lockdrop_incentives
        .multiply_ratio(user_info.total_native_locked, state.total_native_locked);

    let amount_to_transfer = total_incentives - user_info.delegated_incentives;
    let token = Asset::cw20(
        deps.api.addr_validate(&config.incentive_token)?,
        amount_to_transfer,
    );
    let transfer_msg = token.transfer_msg(user_address.clone())?;
    user_info.lockdrop_claimed = true;

    USER_INFO.save(deps.storage, &user_address, &user_info)?;

    Ok(Response::new()
        .add_attribute("action", "claim_rewards")
        .add_attribute("amount", amount_to_transfer)
        .add_message(transfer_msg))
}

fn execute_withdraw_proceeds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let recipient = recipient.unwrap_or_else(|| info.sender.to_string());
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;
    // CHECK :: Only Owner can call this function
    require(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    // CHECK :: Lockdrop withdrawal window should be closed
    let current_timestamp = env.block.time.seconds();
    require(
        current_timestamp >= config.init_timestamp && !is_withdraw_open(current_timestamp, &config),
        ContractError::InvalidWithdrawal {
            msg: Some("Lockdrop withdrawals haven't concluded yet".to_string()),
        },
    )?;

    let native_token = Asset::native(config.native_denom, state.total_native_locked);

    let balance = native_token
        .info
        .query_balance(&deps.querier, env.contract.address)?;
    require(
        balance >= state.total_native_locked,
        ContractError::InvalidWithdrawal {
            msg: Some("Already withdrew funds".to_string()),
        },
    )?;

    let transfer_msg = native_token.transfer_msg(recipient)?;

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "withdraw_proceeds")
        .add_attribute("amount", state.total_native_locked)
        .add_attribute("timestamp", env.block.time.seconds().to_string()))
}

//----------------------------------------------------------------------------------------
// Query Functions
//----------------------------------------------------------------------------------------

/// @dev Returns the contract's configuration
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let contract = ADOContract::default();
    let app_contract = contract.get_app_contract(deps.storage)?;
    let bootstrap_contract_address = config
        .bootstrap_contract_address
        .map(|a| a.get_address(deps.api, &deps.querier, app_contract))
        // Flip Option<Result> to Result<Option>
        .map_or(Ok(None), |v| v.map(Some));

    Ok(ConfigResponse {
        bootstrap_contract_address: bootstrap_contract_address?,
        init_timestamp: config.init_timestamp,
        deposit_window: config.deposit_window,
        withdrawal_window: config.withdrawal_window,
        lockdrop_incentives: config.lockdrop_incentives,
        incentive_token: config.incentive_token,
        native_denom: config.native_denom,
    })
}

/// @dev Returns the contract's Global State
pub fn query_state(deps: Deps) -> Result<StateResponse, ContractError> {
    let state: State = STATE.load(deps.storage)?;
    Ok(StateResponse {
        total_native_locked: state.total_native_locked,
        total_delegated: state.total_delegated,
        are_claims_allowed: state.are_claims_allowed,
    })
}

/// @dev Returns summarized details regarding the user
/// @params user_address : User address whose state is being queries
pub fn query_user_info(
    deps: Deps,
    _env: Env,
    user_address_: String,
) -> Result<UserInfoResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let user_address = deps.api.addr_validate(&user_address_)?;
    let state: State = STATE.load(deps.storage)?;
    let user_info = USER_INFO
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    let total_incentives = config
        .lockdrop_incentives
        .multiply_ratio(user_info.total_native_locked, state.total_native_locked);

    Ok(UserInfoResponse {
        total_native_locked: user_info.total_native_locked,
        total_incentives,
        delegated_incentives: user_info.delegated_incentives,
        is_lockdrop_claimed: user_info.lockdrop_claimed,
        withdrawal_flag: user_info.withdrawal_flag,
    })
}

/// @dev Returns max withdrawable % for a position
pub fn query_max_withdrawable_percent(
    deps: Deps,
    env: Env,
    timestamp: Option<u64>,
) -> Result<Decimal, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(match timestamp {
        Some(timestamp) => allowed_withdrawal_percent(timestamp, &config),
        None => allowed_withdrawal_percent(env.block.time.seconds(), &config),
    })
}

//----------------------------------------------------------------------------------------
// HELPERS
//----------------------------------------------------------------------------------------

/// @dev Returns true if deposits are allowed
fn is_deposit_open(current_timestamp: u64, config: &Config) -> bool {
    let deposits_opened_till = config.init_timestamp + config.deposit_window;
    (current_timestamp >= config.init_timestamp) && (deposits_opened_till >= current_timestamp)
}

/// @dev Returns true if withdrawals are allowed
fn is_withdraw_open(current_timestamp: u64, config: &Config) -> bool {
    let withdrawals_opened_till =
        config.init_timestamp + config.deposit_window + config.withdrawal_window;
    (current_timestamp >= config.init_timestamp) && (withdrawals_opened_till >= current_timestamp)
}

/// @dev Helper function to calculate maximum % of NATIVE deposited that can be withdrawn
/// @params current_timestamp : Current block timestamp
/// @params config : Contract configuration
fn allowed_withdrawal_percent(current_timestamp: u64, config: &Config) -> Decimal {
    let withdrawal_cutoff_init_point = config.init_timestamp + config.deposit_window;

    // Deposit window :: 100% withdrawals allowed
    if current_timestamp < withdrawal_cutoff_init_point {
        return Decimal::from_ratio(100u32, 100u32);
    }

    let withdrawal_cutoff_second_point =
        withdrawal_cutoff_init_point + (config.withdrawal_window / 2u64);
    // Deposit window closed, 1st half of withdrawal window :: 50% withdrawals allowed
    if current_timestamp <= withdrawal_cutoff_second_point {
        return Decimal::from_ratio(50u32, 100u32);
    }

    // max withdrawal allowed decreasing linearly from 50% to 0% vs time elapsed
    let withdrawal_cutoff_final = withdrawal_cutoff_init_point + config.withdrawal_window;
    //  Deposit window closed, 2nd half of withdrawal window :: max withdrawal allowed decreases linearly from 50% to 0% vs time elapsed
    if current_timestamp < withdrawal_cutoff_final {
        let time_left = withdrawal_cutoff_final - current_timestamp;
        Decimal::from_ratio(
            50u64 * time_left,
            100u64 * (withdrawal_cutoff_final - withdrawal_cutoff_second_point),
        )
    }
    // Withdrawals not allowed
    else {
        Decimal::from_ratio(0u32, 100u32)
    }
}
