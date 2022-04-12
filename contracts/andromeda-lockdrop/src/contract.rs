use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut,
    Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Uint128,
    WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_asset::Asset;

use andromeda_protocol::lockdrop::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, LockupInfoQueryData,
    LockupInfoResponse, MigrateMsg, QueryMsg, StateResponse, UpdateConfigMsg, UserInfoResponse,
};

use crate::state::{Config, State, UserInfo, CONFIG, STATE, USER_INFO};

const UUSD_DENOM: &str = "uusd";

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
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // CHECK :: init_timestamp needs to be valid
    if msg.init_timestamp < env.block.time.seconds() {
        return Err(StdError::generic_err(format!(
            "Invalid timestamp. Current timestamp : {}",
            env.block.time.seconds()
        )));
    }

    // CHECK :: deposit_window,withdrawal_window need to be valid (withdrawal_window < deposit_window)
    if msg.deposit_window == 0u64
        || msg.withdrawal_window == 0u64
        || msg.deposit_window <= msg.withdrawal_window
    {
        return Err(StdError::generic_err("Invalid deposit / withdraw window"));
    }

    // CHECK :: init_timestamp needs to be valid
    if msg.seconds_per_duration_unit == 0u64 {
        return Err(StdError::generic_err(
            "seconds_per_duration_unit cannot be 0",
        ));
    }

    let config = Config {
        auction_contract_address: None,
        init_timestamp: msg.init_timestamp,
        deposit_window: msg.deposit_window,
        withdrawal_window: msg.withdrawal_window,
        seconds_per_duration_unit: msg.seconds_per_duration_unit,
        lockdrop_incentives: Uint128::zero(),
        incentive_token: msg.incentive_token,
    };

    let state = State {
        final_ust_locked: Uint128::zero(),
        final_maust_locked: Uint128::zero(),
        total_ust_locked: Uint128::zero(),
        total_mars_delegated: Uint128::zero(),
        are_claims_allowed: false,
    };

    CONFIG.save(deps.storage, &config)?;
    STATE.save(deps.storage, &state)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig { new_config } => update_config(deps, env, info, new_config),
        ExecuteMsg::DepositUst {} => try_deposit_ust(deps, env, info),
        ExecuteMsg::WithdrawUst { amount } => try_withdraw_ust(deps, env, info, amount),
        /*ExecuteMsg::DepositMarsToAuction { amount } => {
            handle_deposit_mars_to_auction(deps, env, info, amount)
        }*/
        ExecuteMsg::EnableClaims {} => handle_enable_claims(deps, env, info),
        ExecuteMsg::ClaimRewards {} => handle_claim_rewards(deps, env, info),
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
) -> Result<Response, StdError> {
    // CHECK :: Tokens sent > 0
    if cw20_msg.amount == Uint128::zero() {
        return Err(StdError::generic_err(
            "Number of tokens sent should be > 0 ",
        ));
    }

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::IncreaseIncentives {} => {
            handle_increase_incentives(deps, env, info, cw20_msg.amount)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::UserInfo { address } => to_binary(&query_user_info(deps, env, address)?),
        QueryMsg::WithdrawalPercentAllowed { timestamp } => {
            to_binary(&query_max_withdrawable_percent(deps, env, timestamp)?)
        }
    }
}

//----------------------------------------------------------------------------------------
// Handle Functions
//----------------------------------------------------------------------------------------

/// @dev Facilitates increasing token incentives that are to be distributed as Lockdrop participation reward
/// @params amount : Number of MARS tokens which are to be added to current incentives
pub fn handle_increase_incentives(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, StdError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.incentive_token {
        return Err(StdError::generic_err("Only incentive tokens are received!"));
    }

    if env.block.time.seconds()
        >= config.init_timestamp + config.deposit_window + config.withdrawal_window
    {
        return Err(StdError::generic_err("Token is already being distributed"));
    };

    config.lockdrop_incentives += amount;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "incentives_increased")
        .add_attribute("amount", amount))
}

/// @dev ADMIN Function. Facilitates state update. Will be used to set address_provider / maUST token address most probably, based on deployment schedule
/// @params new_config : New configuration struct
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfigMsg,
) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;

    if new_config.auction_contract_address.is_some() {
        config.auction_contract_address = Some(
            deps.api
                .addr_validate(&new_config.auction_contract_address.unwrap())?,
        );
    }

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "lockdrop::ExecuteMsg::UpdateConfig"))
}

/// @dev Facilitates UST deposits locked for selected number of weeks
/// @param duration : Number of weeks for which UST will be locked
pub fn try_deposit_ust(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let depositor_address = info.sender;

    // CHECK :: Lockdrop deposit window open
    if !is_deposit_open(env.block.time.seconds(), &config) {
        return Err(StdError::generic_err("Deposit window closed"));
    }

    // Check if multiple native coins sent by the user
    if info.funds.len() > 1 {
        return Err(StdError::generic_err("Trying to deposit several coins"));
    }

    let native_token = info.funds.first().unwrap();
    if native_token.denom != UUSD_DENOM {
        return Err(StdError::generic_err(
            "Only UST among native tokens accepted",
        ));
    }
    // CHECK ::: Amount needs to be valid
    if native_token.amount.is_zero() {
        return Err(StdError::generic_err("Amount must be greater than 0"));
    }

    // USER INFO :: RETRIEVE --> UPDATE
    let mut user_info = USER_INFO
        .may_load(deps.storage, &depositor_address)?
        .unwrap_or_default();

    user_info.total_ust_locked += native_token.amount;

    // STATE :: UPDATE --> SAVE
    state.total_ust_locked += native_token.amount;

    STATE.save(deps.storage, &state)?;
    USER_INFO.save(deps.storage, &depositor_address, &user_info)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "lockdrop::ExecuteMsg::lock_ust"),
        ("user", &depositor_address.to_string()),
        ("ust_deposited", native_token.amount.to_string().as_str()),
    ]))
}

/// @dev Facilitates UST withdrawal from an existing Lockup position. Can only be called when deposit / withdrawal window is open
/// @param duration : Duration of the lockup position from which withdrawal is to be made
/// @param withdraw_amount :  UST amount to be withdrawn
pub fn try_withdraw_ust(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    withdraw_amount: Uint128,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let mut user_info = USER_INFO.load(deps.storage, &info.sender)?;

    // USER ADDRESS AND LOCKUP DETAILS
    let withdrawer_address = info.sender;

    // CHECK :: Lockdrop withdrawal window open
    if !is_withdraw_open(env.block.time.seconds(), &config) {
        return Err(StdError::generic_err("Withdrawals not allowed"));
    }

    // Check :: Amount should be within the allowed withdrawal limit bounds
    let max_withdrawal_percent = allowed_withdrawal_percent(env.block.time.seconds(), &config);
    let max_withdrawal_allowed = user_info.total_ust_locked * max_withdrawal_percent;
    if withdraw_amount > max_withdrawal_allowed {
        return Err(StdError::generic_err(format!(
            "Amount exceeds maximum allowed withdrawal limit of {} ",
            max_withdrawal_allowed
        )));
    }

    // Update withdrawal flag after the deposit window
    if env.block.time.seconds() >= config.init_timestamp + config.deposit_window {
        // CHECK :: Max 1 withdrawal allowed
        if user_info.withdrawal_flag {
            return Err(StdError::generic_err("Max 1 withdrawal allowed"));
        }

        user_info.withdrawal_flag = true;
    }

    user_info.total_ust_locked -= withdraw_amount;

    USER_INFO.save(deps.storage, &withdrawer_address, &user_info)?;

    // STATE :: UPDATE --> SAVE
    state.total_ust_locked -= withdraw_amount;
    STATE.save(deps.storage, &state)?;

    // COSMOS_MSG ::TRANSFER WITHDRAWN UST
    let uusd_token = Asset::native(UUSD_DENOM, withdraw_amount);
    let withdraw_msg = uusd_token.transfer_msg(withdrawer_address.clone())?;

    Ok(Response::new()
        .add_messages(vec![withdraw_msg])
        .add_attributes(vec![
            ("action", "lockdrop::ExecuteMsg::withdraw_ust"),
            ("user", &withdrawer_address.to_string()),
            ("ust_withdrawn", withdraw_amount.to_string().as_str()),
        ]))
}

/// @dev Function callable only by Auction contract to enable MARS Claims by users. Called along-with Bootstrap Auction contract's LP Pool provide liquidity tx
pub fn handle_enable_claims(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    // CHECK :: Auction contract should be set
    if config.auction_contract_address.is_none() {
        return Err(StdError::generic_err("Auction address in lockdrop not set"));
    }

    // CHECK :: ONLY AUCTION CONTRACT CAN CALL THIS FUNCTION
    if info.sender != config.auction_contract_address.clone().unwrap() {
        return Err(StdError::generic_err("Unauthorized"));
    }

    // CHECK :: Claims can only be enabled after the deposit / withdrawal windows are closed
    if is_withdraw_open(env.block.time.seconds(), &config) {
        return Err(StdError::generic_err(
            "Claims can only be enabled after the deposit / withdrawal windows are closed",
        ));
    }

    // CHECK ::: Claims are only enabled once
    if state.are_claims_allowed {
        return Err(StdError::generic_err("Already allowed"));
    }
    state.are_claims_allowed = true;

    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", "Lockdrop::ExecuteMsg::EnableClaims"))
}

/*/// @dev Function to delegate part of the MARS rewards to be used for LP Bootstrapping via auction
/// @param amount : Number of MARS to delegate
pub fn handle_deposit_mars_to_auction(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let user_address = info.sender.clone();

    // CHECK :: Have the deposit / withdraw windows concluded
    if env.block.time.seconds()
        < (config.init_timestamp + config.deposit_window + config.withdrawal_window)
    {
        return Err(StdError::generic_err(
            "Deposit / withdraw windows not closed yet",
        ));
    }

    // CHECK :: Can users withdraw their MARS tokens ? -> if so, then delegation is no longer allowed
    if state.are_claims_allowed {
        return Err(StdError::generic_err("Auction deposits no longer possible"));
    }

    // CHECK :: Address provider should be set
    if config.address_provider.is_none() {
        return Err(StdError::generic_err("Address provider not set"));
    }

    // CHECK :: Auction contract address should be set
    if config.auction_contract_address.is_none() {
        return Err(StdError::generic_err("Auction contract address not set"));
    }

    let mut user_info = USER_INFO
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    // CHECK :: User needs to have atleast 1 lockup position
    if user_info.lockup_positions.is_empty() {
        return Err(StdError::generic_err("No valid lockup positions"));
    }

    // Init response
    let mut response =
        Response::new().add_attribute("action", "Auction::ExecuteMsg::DelegateMarsToAuction");

    // If user's total maUST share == 0 :: We update it
    if user_info.total_maust_share.is_zero() {
        user_info.total_maust_share = calculate_ma_ust_share(
            user_info.total_ust_locked,
            state.final_ust_locked,
            state.final_maust_locked,
        );
        response = response.add_attribute(
            "user_total_maust_share",
            user_info.total_maust_share.to_string(),
        );
    }

    // If user's total MARS rewards == 0 :: We update all of the user's lockup positions to calculate MARS rewards
    if user_info.total_mars_incentives == Uint128::zero() {
        user_info.total_mars_incentives = update_mars_rewards_allocated_to_lockup_positions(
            deps.branch(),
            &config,
            &state,
            user_info.clone(),
        )?;
        response = response.add_attribute(
            "user_total_mars_incentives",
            user_info.total_mars_incentives.to_string(),
        );
    }

    // CHECK :: MARS to delegate cannot exceed user's unclaimed MARS balance
    if amount > (user_info.total_mars_incentives - user_info.delegated_mars_incentives) {
        return Err(StdError::generic_err(format!("Amount cannot exceed user's unclaimed MARS balance. MARS to delegate = {}, Max delegatable MARS = {} ",amount, (user_info.total_mars_incentives - user_info.delegated_mars_incentives))));
    }

    // UPDATE STATE
    user_info.delegated_mars_incentives += amount;
    state.total_mars_delegated += amount;

    // SAVE UPDATED STATE
    STATE.save(deps.storage, &state)?;
    USER_INFO.save(deps.storage, &user_address, &user_info)?;

    let mars_token_address = query_address(
        &deps.querier,
        config.address_provider.unwrap(),
        MarsContract::MarsToken,
    )?;

    // COSMOS_MSG ::Delegate MARS to the LP Bootstrapping via Auction contract
    let delegate_msg = build_send_cw20_token_msg(
        config.auction_contract_address.unwrap().to_string(),
        mars_token_address.to_string(),
        amount,
        to_binary(&AuctionCw20HookMsg::DepositMarsTokens {
            user_address: info.sender,
        })?,
    )?;
    response = response
        .add_message(delegate_msg)
        .add_attribute("user_address", &user_address.to_string())
        .add_attribute("delegated_mars", amount.to_string());

    Ok(response)
}*/

/// @dev Function to claim Rewards and optionally unlock a lockup position (either naturally or forcefully). Claims pending incentives (xMARS) internally and accounts for them via the index updates
/// @params lockup_to_unlock_duration : Duration of the lockup to be unlocked. If 0 then no lockup is to be unlocked
pub fn handle_claim_rewards(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    let user_address = info.sender;
    let mut user_info = USER_INFO
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    let mut response = Response::new().add_attribute(
        "action",
        "Auction::ExecuteMsg::ClaimRewardsAndUnlockPosition",
    );

    // CHECKS ::
    // 2. Valid lockup positions available ?
    // 3. Are claims allowed
    if user_info.lockdrop_claimed {
        return Err(StdError::generic_err("Lockdrop claimed"));
    }
    if user_info.total_ust_locked == Uint128::zero() {
        return Err(StdError::generic_err("No lockup to claim rewards for"));
    }
    if !state.are_claims_allowed {
        return Err(StdError::generic_err("Claim not allowed"));
    }

    // If user's total MARS rewards == 0 :: We update all of the user's lockup positions to calculate MARS rewards
    if user_info.total_incentives.is_zero() {
        user_info.total_incentives = config
            .lockdrop_incentives
            .multiply_ratio(user_info.total_ust_locked, state.total_ust_locked);
        response = response.add_attribute(
            "user_total_incentives",
            user_info.total_incentives.to_string(),
        );
    }

    let amount_to_transfer = user_info.total_incentives - user_info.delegated_mars_incentives;
    let token = Asset::cw20(
        deps.api.addr_validate(&config.incentive_token)?,
        amount_to_transfer,
    );
    let transfer_msg = token.transfer_msg(user_address.clone())?;
    user_info.lockdrop_claimed = true;

    USER_INFO.save(deps.storage, &user_address, &user_info)?;
    Ok(response.add_message(transfer_msg))
}

//----------------------------------------------------------------------------------------
// Query Functions
//----------------------------------------------------------------------------------------

/// @dev Returns the contract's configuration
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        auction_contract_address: config.auction_contract_address,
        init_timestamp: config.init_timestamp,
        deposit_window: config.deposit_window,
        withdrawal_window: config.withdrawal_window,
        seconds_per_duration_unit: config.seconds_per_duration_unit,
        lockdrop_incentives: config.lockdrop_incentives,
    })
}

/// @dev Returns the contract's Global State
pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state: State = STATE.load(deps.storage)?;
    Ok(StateResponse {
        final_ust_locked: state.final_ust_locked,
        total_ust_locked: state.total_ust_locked,
        total_mars_delegated: state.total_mars_delegated,
        are_claims_allowed: state.are_claims_allowed,
    })
}

/// @dev Returns summarized details regarding the user
/// @params user_address : User address whose state is being queries
pub fn query_user_info(deps: Deps, env: Env, user_address_: String) -> StdResult<UserInfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let user_address = deps.api.addr_validate(&user_address_)?;
    let mut state: State = STATE.load(deps.storage)?;
    let mut user_info = USER_INFO
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    // Calculate user's lockdrop incentive share if not finalized
    if user_info.total_incentives.is_zero() {
        user_info.total_incentives = config
            .lockdrop_incentives
            .multiply_ratio(user_info.total_ust_locked, state.total_ust_locked);
    }

    let mut pending_xmars_to_claim = Uint128::zero();

    Ok(UserInfoResponse {
        total_ust_locked: user_info.total_ust_locked,
        total_mars_incentives: user_info.total_incentives,
        delegated_mars_incentives: user_info.delegated_mars_incentives,
        is_lockdrop_claimed: user_info.lockdrop_claimed,
    })
}

/// @dev Returns max withdrawable % for a position
pub fn query_max_withdrawable_percent(
    deps: Deps,
    env: Env,
    timestamp: Option<u64>,
) -> StdResult<Decimal> {
    let config = CONFIG.load(deps.storage)?;
    let max_withdrawable_percent: Decimal;

    match timestamp {
        Some(timestamp) => {
            max_withdrawable_percent = allowed_withdrawal_percent(timestamp, &config);
        }
        None => {
            max_withdrawable_percent =
                allowed_withdrawal_percent(env.block.time.seconds(), &config);
        }
    }

    Ok(max_withdrawable_percent)
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

/// @dev Returns the timestamp when the lockup will get unlocked
fn calculate_unlock_timestamp(config: &Config, duration: u64) -> u64 {
    config.init_timestamp
        + config.deposit_window
        + config.withdrawal_window
        + (duration * config.seconds_per_duration_unit)
}

/// @dev Helper function to calculate maximum % of UST deposited that can be withdrawn
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
