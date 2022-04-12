use std::ops::Mul;

use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut,
    Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Uint128,
    WasmMsg, WasmQuery,
};

use cw20::Cw20ReceiveMsg;

use cw2::set_contract_version;

use andromeda_protocol::lockdrop::{
    CallbackMsg, ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, LockupInfoQueryData,
    LockupInfoResponse, MigrateMsg, QueryMsg, StateResponse, UpdateConfigMsg, UserInfoResponse,
};

use crate::state::{Config, State, UserInfo, CONFIG, LOCKUP_INFO, STATE, USER_INFO};

const UUSD_DENOM: &str = "uusd";

// version info for migration info
const CONTRACT_NAME: &str = "mars_lockdrop";
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

    let mut config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        address_provider: None,
        ma_ust_token: None,
        auction_contract_address: None,
        init_timestamp: msg.init_timestamp,
        deposit_window: msg.deposit_window,
        withdrawal_window: msg.withdrawal_window,
        lockup_durations: msg.lockup_durations,
        seconds_per_duration_unit: msg.seconds_per_duration_unit,
        lockdrop_incentives: Uint128::zero(),
    };

    if msg.address_provider.is_some() {
        config.address_provider = Some(deps.api.addr_validate(&msg.address_provider.unwrap())?);
    }
    if msg.ma_ust_token.is_some() {
        config.ma_ust_token = Some(deps.api.addr_validate(&msg.ma_ust_token.unwrap())?);
    }

    let state = State {
        final_ust_locked: Uint128::zero(),
        final_maust_locked: Uint128::zero(),
        total_ust_locked: Uint128::zero(),
        total_maust_locked: Uint128::zero(),
        total_deposits_weight: Uint128::zero(),
        total_mars_delegated: Uint128::zero(),
        are_claims_allowed: false,
        xmars_rewards_index: Decimal::zero(),
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
        ExecuteMsg::DepositUst { duration } => try_deposit_ust(deps, env, info, duration),
        ExecuteMsg::WithdrawUst { duration, amount } => {
            try_withdraw_ust(deps, env, info, duration, amount)
        }
        ExecuteMsg::DepositMarsToAuction { amount } => {
            handle_deposit_mars_to_auction(deps, env, info, amount)
        }
        ExecuteMsg::EnableClaims {} => handle_enable_claims(deps, env, info),
        ExecuteMsg::DepositUstInRedBank {} => try_deposit_in_red_bank(deps, env, info),
        ExecuteMsg::ClaimRewardsAndUnlock {
            lockup_to_unlock_duration,
        } => handle_claim_rewards_and_unlock_position(deps, env, info, lockup_to_unlock_duration),
        ExecuteMsg::Callback(msg) => _handle_callback(deps, env, info, msg),
    }
}

fn _handle_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> StdResult<Response> {
    // Callback functions can only be called this contract itself
    if info.sender != env.contract.address {
        return Err(StdError::generic_err(
            "callbacks cannot be invoked externally",
        ));
    }
    match msg {
        CallbackMsg::UpdateStateOnRedBankDeposit {
            prev_ma_ust_balance,
        } => update_state_on_red_bank_deposit(deps, env, prev_ma_ust_balance),
        CallbackMsg::UpdateStateOnClaim {
            user,
            prev_xmars_balance,
        } => update_state_on_claim(deps, env, user, prev_xmars_balance),
        CallbackMsg::DissolvePosition { user, duration } => {
            try_dissolve_position(deps, env, user, duration)
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
) -> Result<Response, StdError> {
    // CHECK :: Tokens sent > 0
    if cw20_msg.amount == Uint128::zero() {
        return Err(StdError::generic_err(
            "Number of tokens sent should be > 0 ",
        ));
    }

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::IncreaseMarsIncentives {} => {
            handle_increase_mars_incentives(deps, env, info, cw20_msg.amount)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::UserInfo { address } => to_binary(&query_user_info(deps, env, address)?),
        QueryMsg::LockupInfo { address, duration } => {
            to_binary(&query_lockup_info(deps, address, duration)?)
        }
        QueryMsg::LockupInfoWithId { lockup_id } => {
            to_binary(&query_lockup_info_with_id(deps, lockup_id)?)
        }
        QueryMsg::WithdrawalPercentAllowed { timestamp } => {
            to_binary(&query_max_withdrawable_percent(deps, env, timestamp)?)
        }
    }
}

//----------------------------------------------------------------------------------------
// Handle Functions
//----------------------------------------------------------------------------------------

/// @dev Facilitates increasing MARS incentives that are to be distributed as Lockdrop participation reward
/// @params amount : Number of MARS tokens which are to be added to current incentives
pub fn handle_increase_mars_incentives(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, StdError> {
    let mut config = CONFIG.load(deps.storage)?;

    if config.address_provider.is_none() {
        return Err(StdError::generic_err("Address provider not set"));
    }

    let mars_token_address = query_address(
        &deps.querier,
        config.address_provider.clone().unwrap(),
        MarsContract::MarsToken,
    )?;

    if info.sender != mars_token_address {
        return Err(StdError::generic_err("Only mars tokens are received!"));
    }

    if env.block.time.seconds()
        >= config.init_timestamp + config.deposit_window + config.withdrawal_window
    {
        return Err(StdError::generic_err("MARS is already being distributed"));
    };

    config.lockdrop_incentives += amount;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "mars_incentives_increased")
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
    if info.sender != config.owner {
        return Err(StdError::generic_err("Only owner can update configuration"));
    }

    // UPDATE :: ADDRESSES IF PROVIDED
    if new_config.address_provider.is_some() {
        config.address_provider = Some(
            deps.api
                .addr_validate(&new_config.address_provider.unwrap())?,
        );
    }
    if new_config.ma_ust_token.is_some() {
        config.ma_ust_token = Some(deps.api.addr_validate(&new_config.ma_ust_token.unwrap())?);
    }
    if new_config.auction_contract_address.is_some() {
        config.auction_contract_address = Some(
            deps.api
                .addr_validate(&new_config.auction_contract_address.unwrap())?,
        );
    }
    if new_config.owner.is_some() {
        config.owner = deps.api.addr_validate(&new_config.owner.unwrap())?;
    }

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "lockdrop::ExecuteMsg::UpdateConfig"))
}

/// @dev Facilitates UST deposits locked for selected number of weeks
/// @param duration : Number of weeks for which UST will be locked
pub fn try_deposit_ust(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let depositor_address = info.sender.clone();

    // CHECK :: Lockdrop deposit window open
    if !is_deposit_open(env.block.time.seconds(), &config) {
        return Err(StdError::generic_err("Deposit window closed"));
    }

    // Check if multiple native coins sent by the user
    if info.funds.len() > 1 {
        return Err(StdError::generic_err("Trying to deposit several coins"));
    }

    let native_token = info.funds.first().unwrap();
    if native_token.denom != *UUSD_DENOM {
        return Err(StdError::generic_err(
            "Only UST among native tokens accepted",
        ));
    }
    // CHECK ::: Amount needs to be valid
    if native_token.amount.is_zero() {
        return Err(StdError::generic_err("Amount must be greater than 0"));
    }

    let deposit_weight = calculate_weight(native_token.amount, duration, &config)?;

    // LOCKUP INFO :: RETRIEVE --> UPDATE
    let lockup_id = depositor_address.to_string() + &duration.to_string();
    let mut lockup_info = LOCKUP_INFO
        .may_load(deps.storage, lockup_id.as_bytes())?
        .unwrap_or_default();

    lockup_info.ust_locked += native_token.amount;

    // USER INFO :: RETRIEVE --> UPDATE
    let mut user_info = USER_INFO
        .may_load(deps.storage, &depositor_address)?
        .unwrap_or_default();

    user_info.total_ust_locked += native_token.amount;

    if lockup_info.duration == 0u64 {
        lockup_info.duration = duration;
        lockup_info.unlock_timestamp = calculate_unlock_timestamp(&config, duration);
        user_info.lockup_positions.push(lockup_id.clone());
    }

    // STATE :: UPDATE --> SAVE
    state.total_ust_locked += native_token.amount;
    state.total_deposits_weight += deposit_weight;

    STATE.save(deps.storage, &state)?;
    LOCKUP_INFO.save(deps.storage, lockup_id.as_bytes(), &lockup_info)?;
    USER_INFO.save(deps.storage, &depositor_address, &user_info)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "lockdrop::ExecuteMsg::lock_ust"),
        ("user", &depositor_address.to_string()),
        ("duration", duration.to_string().as_str()),
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
    duration: u64,
    withdraw_amount: Uint128,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    // USER ADDRESS AND LOCKUP DETAILS
    let withdrawer_address = info.sender;
    let lockup_id = withdrawer_address.to_string() + &duration.to_string();
    let mut lockup_info = LOCKUP_INFO
        .may_load(deps.storage, lockup_id.as_bytes())?
        .unwrap_or_default();

    // CHECK :: Lockdrop withdrawal window open
    if !is_withdraw_open(env.block.time.seconds(), &config) {
        return Err(StdError::generic_err("Withdrawals not allowed"));
    }

    // CHECK :: Valid Lockup
    if lockup_info.ust_locked.is_zero() {
        return Err(StdError::generic_err("Lockup doesn't exist"));
    }

    // Check :: Amount should be within the allowed withdrawal limit bounds
    let max_withdrawal_percent = allowed_withdrawal_percent(env.block.time.seconds(), &config);
    let max_withdrawal_allowed = lockup_info.ust_locked * max_withdrawal_percent;
    if withdraw_amount > max_withdrawal_allowed {
        return Err(StdError::generic_err(format!(
            "Amount exceeds maximum allowed withdrawal limit of {} ",
            max_withdrawal_allowed
        )));
    }

    // Update withdrawal flag after the deposit window
    if env.block.time.seconds() >= config.init_timestamp + config.deposit_window {
        // CHECK :: Max 1 withdrawal allowed
        if lockup_info.withdrawal_flag {
            return Err(StdError::generic_err("Max 1 withdrawal allowed"));
        }

        lockup_info.withdrawal_flag = true;
    }

    // LOCKUP INFO :: RETRIEVE --> UPDATE
    lockup_info.ust_locked -= withdraw_amount;

    // USER INFO :: RETRIEVE --> UPDATE
    let mut user_info = USER_INFO
        .may_load(deps.storage, &withdrawer_address)?
        .unwrap_or_default();

    user_info.total_ust_locked -= withdraw_amount;
    if lockup_info.ust_locked == Uint128::zero() {
        remove_lockup_pos_from_user_info(&mut user_info, lockup_id.clone())?;
        LOCKUP_INFO.remove(deps.storage, lockup_id.as_bytes());
    } else {
        LOCKUP_INFO.save(deps.storage, lockup_id.as_bytes(), &lockup_info)?;
    }
    USER_INFO.save(deps.storage, &withdrawer_address, &user_info)?;

    // STATE :: UPDATE --> SAVE
    state.total_ust_locked -= withdraw_amount;
    state.total_deposits_weight -= calculate_weight(withdraw_amount, duration, &config)?;
    STATE.save(deps.storage, &state)?;

    // COSMOS_MSG ::TRANSFER WITHDRAWN UST
    let withdraw_msg = build_send_native_asset_msg(
        deps.as_ref(),
        withdrawer_address.clone(),
        UUSD_DENOM,
        withdraw_amount.into(),
    )?;

    Ok(Response::new()
        .add_messages(vec![withdraw_msg])
        .add_attributes(vec![
            ("action", "lockdrop::ExecuteMsg::withdraw_ust"),
            ("user", &withdrawer_address.to_string()),
            ("duration", duration.to_string().as_str()),
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

/// @dev Admin Function. Deposits all UST into the Red Bank
pub fn try_deposit_in_red_bank(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    // CHECK :: Only Owner can call this function
    if info.sender != config.owner {
        return Err(StdError::generic_err("Unauthorized"));
    }

    // CHECK :: Address provider should be set
    if config.address_provider.is_none() {
        return Err(StdError::generic_err("Address provider not set"));
    }

    // CHECK :: maUST address should be set
    if config.ma_ust_token.is_none() {
        return Err(StdError::generic_err("maUST not set"));
    }

    // CHECK :: Lockdrop withdrawal window should be closed
    if env.block.time.seconds() < config.init_timestamp
        || is_withdraw_open(env.block.time.seconds(), &config)
    {
        return Err(StdError::generic_err(
            "Lockdrop withdrawals haven't concluded yet",
        ));
    }

    // CHECK :: Revert in-case funds have already been deposited in red-bank
    if state.final_maust_locked > Uint128::zero() {
        return Err(StdError::generic_err("Already deposited"));
    }

    // FETCH CURRENT BALANCES (UST / maUST), PREPARE DEPOSIT MSG
    let red_bank = query_address(
        &deps.querier,
        config.address_provider.unwrap(),
        MarsContract::RedBank,
    )?;
    let ma_ust_balance = cw20_get_balance(
        &deps.querier,
        config.ma_ust_token.unwrap(),
        env.contract.address.clone(),
    )?;

    // COSMOS_MSG :: DEPOSIT UST IN RED BANK
    let deposit_msg = build_deposit_into_redbank_msg(
        deps.as_ref(),
        red_bank,
        UUSD_DENOM.to_string(),
        state.total_ust_locked,
    )?;

    // COSMOS_MSG :: UPDATE CONTRACT STATE
    let update_state_msg = CallbackMsg::UpdateStateOnRedBankDeposit {
        prev_ma_ust_balance: ma_ust_balance,
    }
    .to_cosmos_msg(&env.contract.address)?;

    Ok(Response::new()
        .add_messages(vec![deposit_msg, update_state_msg])
        .add_attributes(vec![
            ("action", "lockdrop::ExecuteMsg::DepositInRedBank"),
            (
                "ust_deposited_in_red_bank",
                state.total_ust_locked.to_string().as_str(),
            ),
            ("timestamp", env.block.time.seconds().to_string().as_str()),
        ]))
}

// @dev Function to delegate part of the MARS rewards to be used for LP Bootstrapping via auction
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
}

/// @dev Function to claim Rewards and optionally unlock a lockup position (either naturally or forcefully). Claims pending incentives (xMARS) internally and accounts for them via the index updates
/// @params lockup_to_unlock_duration : Duration of the lockup to be unlocked. If 0 then no lockup is to be unlocked
pub fn handle_claim_rewards_and_unlock_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lockup_to_unlock_duration_option: Option<u64>,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    // CHECK :: Address provider should be set
    if config.address_provider.is_none() {
        return Err(StdError::generic_err("Address provider not set"));
    }

    let user_address = info.sender;
    let mut user_info = USER_INFO
        .may_load(deps.storage, &user_address)?
        .unwrap_or_default();

    let mut response = Response::new().add_attribute(
        "action",
        "Auction::ExecuteMsg::ClaimRewardsAndUnlockPosition",
    );

    // If a lockup is to be unlocked, then we check that it is a valid lockup position
    if let Some(lockup_to_unlock_duration) = lockup_to_unlock_duration_option {
        let lockup_id = user_address.to_string() + &lockup_to_unlock_duration.to_string();
        let lockup_info = LOCKUP_INFO
            .may_load(deps.storage, lockup_id.as_bytes())?
            .unwrap_or_default();

        if lockup_info.ust_locked == Uint128::zero() {
            return Err(StdError::generic_err("Invalid lockup"));
        }

        if lockup_info.unlock_timestamp > env.block.time.seconds() {
            let time_remaining = lockup_info.unlock_timestamp - env.block.time.seconds();
            return Err(StdError::generic_err(format!(
                "{} seconds to Unlock",
                time_remaining
            )));
        }

        response = response
            .add_attribute("action", "unlock_position")
            .add_attribute("ust_amount", lockup_info.ust_locked.to_string())
            .add_attribute("duration", lockup_info.duration.to_string())
    }

    // CHECKS ::
    // 2. Valid lockup positions available ?
    // 3. Are claims allowed
    if user_info.total_ust_locked == Uint128::zero() {
        return Err(StdError::generic_err("No lockup to claim rewards for"));
    }
    if !state.are_claims_allowed {
        return Err(StdError::generic_err("Claim not allowed"));
    }

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
    if user_info.total_mars_incentives.is_zero() {
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

    // QUERY:: XMARS & Incentives Contract addresses
    let mars_contracts = vec![MarsContract::Incentives, MarsContract::XMarsToken];
    let mut addresses_query = query_addresses(
        &deps.querier.clone(),
        config.address_provider.unwrap(),
        mars_contracts,
    )
    .map_err(|_| StdError::generic_err("mars address provider query failed"))?;
    let xmars_address = addresses_query.pop().unwrap();
    let incentives_address = addresses_query.pop().unwrap();

    // MARS REWARDS :: Query if any rewards to claim and if so, claim them (we receive them as XMARS)
    let mars_unclaimed: Uint128 = query_pending_mars_to_be_claimed(
        &deps.querier,
        incentives_address.to_string(),
        env.contract.address.to_string(),
    )?;
    let xmars_balance =
        cw20_get_balance(&deps.querier, xmars_address, env.contract.address.clone())?;

    if !mars_unclaimed.is_zero() {
        let claim_xmars_msg = build_claim_xmars_rewards(incentives_address)?;
        response = response
            .add_message(claim_xmars_msg)
            .add_attribute("xmars_claimed", "true");
    }

    // CALLBACK ::  UPDATE STATE
    let callback_msg = CallbackMsg::UpdateStateOnClaim {
        user: user_address.clone(),
        prev_xmars_balance: xmars_balance,
    }
    .to_cosmos_msg(&env.contract.address)?;
    response = response.add_message(callback_msg);

    // CALLBACK MSG :: DISSOLVE LOCKUP POSITION
    if let Some(lockup_to_unlock_duration) = lockup_to_unlock_duration_option {
        let callback_dissolve_position_msg = CallbackMsg::DissolvePosition {
            user: user_address.clone(),
            duration: lockup_to_unlock_duration,
        }
        .to_cosmos_msg(&env.contract.address)?;
        response = response.add_message(callback_dissolve_position_msg);
    }

    USER_INFO.save(deps.storage, &user_address, &user_info)?;
    Ok(response)
}

//----------------------------------------------------------------------------------------
// Callback Functions
//----------------------------------------------------------------------------------------

/// @dev Callback function. Updates state after UST is deposited in the Red Bank
/// @params prev_ma_ust_balance : Previous maUST Token balance
pub fn update_state_on_red_bank_deposit(
    deps: DepsMut,
    env: Env,
    prev_ma_ust_balance: Uint128,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let cur_ma_ust_balance = cw20_get_balance(
        &deps.querier,
        config.ma_ust_token.unwrap(),
        env.contract.address,
    )?;
    let m_ust_minted = cur_ma_ust_balance - prev_ma_ust_balance;

    // STATE :: UPDATE --> SAVE
    state.final_ust_locked = state.total_ust_locked;
    state.final_maust_locked = m_ust_minted;

    state.total_ust_locked = Uint128::zero();
    state.total_maust_locked = m_ust_minted;

    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "lockdrop::CallbackMsg::RedBankDeposit"),
        ("maUST_minted", m_ust_minted.to_string().as_str()),
    ]))
}

/// @dev Callback function. Updated indexes (if xMars is claimed), calculates user's Mars rewards (if not already done), and transfers rewards (MARS and xMars) to the user
/// @params user : User address
/// @params prev_xmars_balance : Previous xMars balance. Used to calculate how much xMars was claimed from the incentives contract
pub fn update_state_on_claim(
    deps: DepsMut,
    env: Env,
    user: Addr,
    prev_xmars_balance: Uint128,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?; // Index is updated
    let mut user_info = USER_INFO.may_load(deps.storage, &user)?.unwrap_or_default();

    // QUERY:: xMars and Mars Contract addresses
    let mars_contracts = vec![MarsContract::MarsToken, MarsContract::XMarsToken];
    let mut addresses_query = query_addresses(
        &deps.querier,
        config.address_provider.unwrap(),
        mars_contracts,
    )
    .map_err(|_| StdError::generic_err("mars address provider query failed"))?;

    let xmars_address = addresses_query.pop().unwrap();
    let mars_address = addresses_query.pop().unwrap();

    let mut response = Response::new().add_attribute("user_address", user.to_string());

    // Calculate XMARS Claimed as rewards
    let cur_xmars_balance =
        cw20_get_balance(&deps.querier, xmars_address.clone(), env.contract.address)?;
    let xmas_accrued = cur_xmars_balance - prev_xmars_balance;
    response = response.add_attribute("total_xmars_claimed", xmas_accrued.to_string());

    // UPDATE :: GLOBAL & USER INDEX (XMARS rewards tracker)
    if xmas_accrued > Uint128::zero() {
        update_xmars_rewards_index(&mut state, xmas_accrued);
    }

    // COSMOS MSG :: SEND X-MARS (DEPOSIT INCENTIVES) IF > 0
    let pending_xmars_rewards = compute_user_accrued_reward(&state, &mut user_info);
    if pending_xmars_rewards > Uint128::zero() {
        user_info.total_xmars_claimed += pending_xmars_rewards;

        let transfer_xmars_msg = build_transfer_cw20_token_msg(
            user.clone(),
            xmars_address.to_string(),
            pending_xmars_rewards,
        )?;

        response = response
            .add_message(transfer_xmars_msg)
            .add_attribute("user_xmars_claimed", pending_xmars_rewards.to_string());
    }

    let mars_to_transfer = user_info.total_mars_incentives - user_info.delegated_mars_incentives;

    // COSMOS MSG :: SEND MARS (LOCKDROP REWARD) IF > 0
    if !user_info.lockdrop_claimed && mars_to_transfer > Uint128::zero() {
        let transfer_mars_msg = build_transfer_cw20_token_msg(
            user.clone(),
            mars_address.to_string(),
            mars_to_transfer,
        )?;
        user_info.lockdrop_claimed = true;
        response = response
            .add_message(transfer_mars_msg)
            .add_attribute("user_mars_claimed", mars_to_transfer.to_string());
    }

    // SAVE UPDATED STATES
    STATE.save(deps.storage, &state)?;
    USER_INFO.save(deps.storage, &user, &user_info)?;

    Ok(response)
}

// CALLBACK :: CALLED BY try_unlock_position FUNCTION --> DELETES LOCKUP POSITION
/// @dev  Callback function. Unlocks a lockup position. Either naturally after duration expiration or forcefully by returning MARS (lockdrop incentives)
/// @params user : User address whose position is to be unlocked
/// @params duration :Lockup duration of the position to be unlocked
pub fn try_dissolve_position(
    deps: DepsMut,
    _env: Env,
    user: Addr,
    duration: u64,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let mut user_info = USER_INFO.may_load(deps.storage, &user)?.unwrap_or_default();

    let lockup_id = user.to_string() + &duration.to_string();
    let mut lockup_info = LOCKUP_INFO
        .may_load(deps.storage, lockup_id.as_bytes())?
        .unwrap_or_default();

    let maust_to_withdraw = calculate_ma_ust_share(
        lockup_info.ust_locked,
        state.final_ust_locked,
        state.final_maust_locked,
    );

    // UPDATE STATE
    state.total_maust_locked -= maust_to_withdraw;

    // UPDATE USER INFO
    // user_info.total_ust_locked = user_info.total_ust_locked - lockup_info.ust_locked;
    user_info.total_maust_share -= maust_to_withdraw;

    // DISSOLVE LOCKUP POSITION
    lockup_info.ust_locked = Uint128::zero();
    remove_lockup_pos_from_user_info(&mut user_info, lockup_id.clone())?;

    let mut cosmos_msgs = vec![];

    let maust_transfer_msg = build_transfer_cw20_token_msg(
        user.clone(),
        config.ma_ust_token.unwrap().to_string(),
        maust_to_withdraw,
    )?;
    cosmos_msgs.push(maust_transfer_msg);

    STATE.save(deps.storage, &state)?;
    USER_INFO.save(deps.storage, &user, &user_info)?;
    LOCKUP_INFO.remove(deps.storage, lockup_id.as_bytes());

    Ok(Response::new()
        .add_messages(cosmos_msgs)
        .add_attributes(vec![
            ("action", "lockdrop::Callback::DissolvePosition"),
            ("ma_ust_transferred", maust_to_withdraw.to_string().as_str()),
        ]))
}

//----------------------------------------------------------------------------------------
// Query Functions
//----------------------------------------------------------------------------------------

/// @dev Returns the contract's configuration
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        address_provider: config.address_provider,
        ma_ust_token: config.ma_ust_token,
        auction_contract_address: config.auction_contract_address,
        init_timestamp: config.init_timestamp,
        deposit_window: config.deposit_window,
        withdrawal_window: config.withdrawal_window,
        lockup_durations: config.lockup_durations,
        seconds_per_duration_unit: config.seconds_per_duration_unit,
        lockdrop_incentives: config.lockdrop_incentives,
    })
}

/// @dev Returns the contract's Global State
pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state: State = STATE.load(deps.storage)?;
    Ok(StateResponse {
        final_ust_locked: state.final_ust_locked,
        final_maust_locked: state.final_maust_locked,
        total_ust_locked: state.total_ust_locked,
        total_maust_locked: state.total_maust_locked,
        total_mars_delegated: state.total_mars_delegated,
        are_claims_allowed: state.are_claims_allowed,
        total_deposits_weight: state.total_deposits_weight,
        xmars_rewards_index: state.xmars_rewards_index,
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

    // Calculate user's maUST share if not already done
    if user_info.total_maust_share == Uint128::zero() && state.final_maust_locked != Uint128::zero()
    {
        user_info.total_maust_share = calculate_ma_ust_share(
            user_info.total_ust_locked,
            state.final_ust_locked,
            state.final_maust_locked,
        );
    }

    // Calculate user's lockdrop incentive share if not finalized
    if user_info.total_mars_incentives == Uint128::zero() {
        for lockup_id in user_info.lockup_positions.iter() {
            let lockup_info = LOCKUP_INFO.load(deps.storage, lockup_id.as_bytes())?;

            let position_rewards = calculate_mars_incentives_for_lockup(
                lockup_info.ust_locked,
                lockup_info.duration,
                &config,
                state.total_deposits_weight,
            )?;
            user_info.total_mars_incentives += position_rewards;
        }
    }

    let mut pending_xmars_to_claim = Uint128::zero();

    // QUERY:: Contract addresses
    if config.address_provider.is_some() {
        let mars_contracts = vec![MarsContract::Incentives];
        let mut addresses_query = query_addresses(
            &deps.querier,
            config.address_provider.unwrap(),
            mars_contracts,
        )
        .map_err(|_| StdError::generic_err("mars address provider query failed"))?;
        let incentives_address = addresses_query.pop().unwrap();

        // QUERY :: XMARS REWARDS TO BE CLAIMED  ?
        let xmas_accrued: Uint128 = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: incentives_address.to_string(),
            msg: to_binary(&IncentivesQueryMsg::UserUnclaimedRewards {
                user_address: env.contract.address.to_string(),
            })?,
        }))?;

        update_xmars_rewards_index(&mut state, xmas_accrued);
        pending_xmars_to_claim = compute_user_accrued_reward(&state, &mut user_info);
    }

    Ok(UserInfoResponse {
        total_ust_locked: user_info.total_ust_locked,
        total_maust_share: user_info.total_maust_share,
        lockup_position_ids: user_info.lockup_positions,
        total_mars_incentives: user_info.total_mars_incentives,
        delegated_mars_incentives: user_info.delegated_mars_incentives,
        is_lockdrop_claimed: user_info.lockdrop_claimed,
        reward_index: user_info.reward_index,
        total_xmars_claimed: user_info.total_xmars_claimed,
        pending_xmars_to_claim,
    })
}

/// @dev Returns summarized details regarding the user
pub fn query_lockup_info(deps: Deps, user: String, duration: u64) -> StdResult<LockupInfoResponse> {
    let lockup_id = user + &duration.to_string();
    query_lockup_info_with_id(deps, lockup_id)
}

/// @dev Returns summarized details regarding the user
pub fn query_lockup_info_with_id(deps: Deps, lockup_id: String) -> StdResult<LockupInfoResponse> {
    let lockup_info_query = LOCKUP_INFO.may_load(deps.storage, lockup_id.as_bytes())?;

    if let Some(lockup_info) = lockup_info_query {
        let state: State = STATE.load(deps.storage)?;
        let mut lockup_info_query_data = LockupInfoQueryData {
            duration: lockup_info.duration,
            ust_locked: lockup_info.ust_locked,
            maust_balance: calculate_ma_ust_share(
                lockup_info.ust_locked,
                state.final_ust_locked,
                state.final_maust_locked,
            ),
            lockdrop_reward: lockup_info.lockdrop_reward,
            unlock_timestamp: lockup_info.unlock_timestamp,
            withdrawal_flag: lockup_info.withdrawal_flag,
        };

        if lockup_info_query_data.lockdrop_reward == Uint128::zero() {
            let config = CONFIG.load(deps.storage)?;
            lockup_info_query_data.lockdrop_reward = calculate_mars_incentives_for_lockup(
                lockup_info_query_data.ust_locked,
                lockup_info_query_data.duration,
                &config,
                state.total_deposits_weight,
            )?;
        }

        Ok(LockupInfoResponse {
            lockup_info: Some(lockup_info_query_data),
        })
    } else {
        Ok(LockupInfoResponse { lockup_info: None })
    }
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

/// @dev Removes lockup position id from user info's lockup position list
/// @params lockup_id : Lockup Id to be removed
fn remove_lockup_pos_from_user_info(user_info: &mut UserInfo, lockup_id: String) -> StdResult<()> {
    let index_search = user_info
        .lockup_positions
        .iter()
        .position(|x| *x == lockup_id);

    if let Some(index) = index_search {
        user_info.lockup_positions.remove(index);
        Ok(())
    } else {
        Err(StdError::generic_err(format!(
            "Lockup position not found for id {}",
            lockup_id
        )))
    }
}

///  @dev Helper function to calculate maximum % of UST deposited that can be withdrawn
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

//-----------------------------
// HELPER FUNCTIONS :: COMPUTATIONS
//-----------------------------

/// @dev Function to calculate & update MARS rewards allocated for each of the user position
/// @params config: configuration struct
/// @params state: state struct
/// @params user_info : user Info struct
/// Returns user's total MARS rewards
fn update_mars_rewards_allocated_to_lockup_positions(
    deps: DepsMut,
    config: &Config,
    state: &State,
    user_info: UserInfo,
) -> StdResult<Uint128> {
    let mut total_mars_rewards = Uint128::zero();

    for lockup_id in user_info.lockup_positions {
        // Retrieve mutable Lockup position
        let mut lockup_info = LOCKUP_INFO
            .load(deps.storage, lockup_id.as_bytes())
            .unwrap();

        let position_rewards = calculate_mars_incentives_for_lockup(
            lockup_info.ust_locked,
            lockup_info.duration,
            config,
            state.total_deposits_weight,
        )?;

        lockup_info.lockdrop_reward = position_rewards;
        total_mars_rewards += position_rewards;
        LOCKUP_INFO.save(deps.storage, lockup_id.as_bytes(), &lockup_info)?;
    }
    Ok(total_mars_rewards)
}

/// @dev Helper function to calculate MARS rewards for a particular Lockup position
/// @params deposited_ust : UST deposited to that particular Lockup position
/// @params duration : Duration of the lockup
/// @params config : Configuration struct
/// @params total_deposits_weight : Total calculated weight of all the UST deposited in the contract
fn calculate_mars_incentives_for_lockup(
    deposited_ust: Uint128,
    duration: u64,
    config: &Config,
    total_deposits_weight: Uint128,
) -> StdResult<Uint128> {
    if total_deposits_weight == Uint128::zero() {
        return Ok(Uint128::zero());
    }
    let amount_weight = calculate_weight(deposited_ust, duration, config)?;
    Ok(config.lockdrop_incentives * Decimal::from_ratio(amount_weight, total_deposits_weight))
}

/// @dev Helper function. Returns effective weight for the amount to be used for calculating lockdrop rewards
/// @params amount : Number of LP tokens
/// @params duration : Selected duration unit
/// @config : Config struct
fn calculate_weight(amount: Uint128, duration: u64, config: &Config) -> StdResult<Uint128> {
    // get boost value for duration
    for lockup_option in config.lockup_durations.iter() {
        if lockup_option.duration == duration {
            return Ok(amount.mul(lockup_option.boost));
        }
    }

    Err(StdError::generic_err(format!(
        "Boost not found for duration {}",
        duration
    )))
}

/// @dev Accrue xMARS rewards by updating the reward index
/// @params state : Global state struct
/// @params xmas_accrued : xMARS tokens claimed as rewards from the incentives contract
fn update_xmars_rewards_index(state: &mut State, xmas_accrued: Uint128) {
    if state.total_maust_locked == Uint128::zero() {
        return;
    }
    let xmars_rewards_index_increment = Decimal::from_ratio(xmas_accrued, state.total_maust_locked);
    state.xmars_rewards_index = state.xmars_rewards_index + xmars_rewards_index_increment;
}

/// @dev Accrue MARS reward for the user by updating the user reward index and and returns the pending rewards (xMars) to be claimed by the user
/// @params state : Global state struct
/// @params user_info : UserInfo struct
fn compute_user_accrued_reward(state: &State, user_info: &mut UserInfo) -> Uint128 {
    if state.final_ust_locked == Uint128::zero() {
        return Uint128::zero();
    }
    let pending_xmars = (user_info.total_maust_share * state.xmars_rewards_index)
        - (user_info.total_maust_share * user_info.reward_index);
    user_info.reward_index = state.xmars_rewards_index;
    pending_xmars
}

/// @dev Returns maUST Token share against UST amount. Calculated as =  (deposited UST / Final UST deposited) * Final maUST Locked
/// @params ust_locked_share : UST amount for which maUST share is to be calculated
/// @params final_ust_locked : Total UST amount which was deposited into Red Bank
/// @params final_maust_locked : Total maUST tokens minted againt the UST deposited into Red Bank
fn calculate_ma_ust_share(
    ust_locked_share: Uint128,
    final_ust_locked: Uint128,
    final_maust_locked: Uint128,
) -> Uint128 {
    if final_ust_locked == Uint128::zero() {
        return Uint128::zero();
    }
    final_maust_locked * Decimal::from_ratio(ust_locked_share, final_ust_locked)
}

//-----------------------------
// QUERY HELPERS
//-----------------------------

/// @dev Helper function. Queries pending Mars to be claimed from the incentives contract
/// @params incentives_address : Incentives contract address
/// @params contract_addr : Address for which pending mars is to be queried
pub fn query_pending_mars_to_be_claimed(
    querier: &QuerierWrapper,
    incentives_address: String,
    contract_addr: String,
) -> StdResult<Uint128> {
    let response = querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: incentives_address,
            msg: to_binary(&IncentivesQueryMsg::UserUnclaimedRewards {
                user_address: contract_addr,
            })
            .unwrap(),
        }))
        .unwrap();
    Ok(response)
}

fn query_address(
    querier: &QuerierWrapper,
    address_provider_address: Addr,
    contract: MarsContract,
) -> StdResult<Addr> {
    let query: Addr = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: address_provider_address.to_string(),
        msg: to_binary(&AddressProviderQueryMsg::Address { contract })?,
    }))?;

    Ok(query)
}

fn query_addresses(
    querier: &QuerierWrapper,
    address_provider_address: Addr,
    contracts: Vec<MarsContract>,
) -> StdResult<Vec<Addr>> {
    let expected_len = contracts.len();

    let query: Vec<Addr> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: address_provider_address.to_string(),
        msg: to_binary(&AddressProviderQueryMsg::Addresses { contracts })?,
    }))?;

    if query.len() != expected_len {
        return Err(StdError::generic_err(format!(
            "Expected {} addresses, got {}",
            query.len(),
            expected_len
        )));
    }

    Ok(query)
}

//-----------------------------
// COSMOS_MSGs
//-----------------------------

/// @dev Helper function. Returns CosmosMsg to deposit UST into the Red Bank
/// @params redbank_address : Red Bank contract address
/// @params denom_stable : uusd stable denom
/// @params amount : UST amount to be deposited
fn build_deposit_into_redbank_msg(
    deps: Deps,
    redbank_address: Addr,
    denom_stable: String,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: redbank_address.to_string(),
        funds: vec![deduct_tax(
            deps,
            Coin {
                denom: denom_stable.to_string(),
                amount,
            },
        )?],
        msg: to_binary(&RedBankExecuteMsg::DepositNative {
            denom: denom_stable,
        })?,
    }))
}

/// @dev Helper function. Returns CosmosMsg to claim xMars rewards from the incentives contract
fn build_claim_xmars_rewards(incentives_contract: Addr) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: incentives_contract.to_string(),
        funds: vec![],
        msg: to_binary(&IncentivesExecuteMsg::ClaimRewards {})?,
    }))
}
