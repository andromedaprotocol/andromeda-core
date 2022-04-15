use crate::state::{Config, GlobalRewardInfo, State, CONFIG, GLOBAL_REWARD_INFOS};
use andromeda_protocol::cw20_staking::RewardToken;
use common::{error::ContractError, require};
use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{Decimal, Storage, Uint128};

pub(crate) fn compute_allocated_rewards(
    storage: &mut dyn Storage,
    config: &mut Config,
    current_timestamp: u64,
    state: &State,
) -> Result<(), ContractError> {
    let allocated_reward_tokens: Vec<&mut RewardToken> = config
        .additional_reward_tokens
        .iter_mut()
        .filter(|r| r.allocation_info.is_some())
        .collect();
    for token in allocated_reward_tokens {
        let mut global_reward_info = GLOBAL_REWARD_INFOS.load(storage, &token.to_string())?;
        compute_allocated_reward(
            state.total_share,
            token,
            &mut global_reward_info,
            current_timestamp,
        )?;
    }
    CONFIG.save(storage, &config)?;
    Ok(())
}

/// @dev Computes total accrued rewards
fn compute_allocated_reward(
    total_share: Uint128,
    reward_token: &mut RewardToken,
    global_reward_info: &mut GlobalRewardInfo,
    cur_timestamp: u64,
) -> Result<(), ContractError> {
    require(
        reward_token.allocation_info.is_some(),
        ContractError::InvalidAsset {
            asset: reward_token.to_string(),
        },
    )?;
    let allocation_info = reward_token.allocation_info.as_mut().unwrap();
    let mut state = &mut allocation_info.state;
    let config = &allocation_info.config;
    // If the reward distribution period is over
    if state.last_distributed == config.till_timestamp {
        return Ok(());
    }

    let mut last_distribution_cycle = state.current_cycle;
    state.current_cycle = calculate_cycles_elapsed(
        cur_timestamp,
        config.init_timestamp,
        config.cycle_duration,
        config.till_timestamp,
    );
    let mut rewards_to_distribute = Uint128::zero();
    let mut last_distribution_next_timestamp: u64; // 0 as u64;

    while state.current_cycle >= last_distribution_cycle {
        last_distribution_next_timestamp = std::cmp::min(
            config.till_timestamp,
            calculate_init_timestamp_for_cycle(
                config.init_timestamp,
                last_distribution_cycle + 1,
                config.cycle_duration,
            ),
        );
        rewards_to_distribute += rewards_distributed_for_cycle(
            Decimal::from_ratio(state.current_cycle_rewards, config.cycle_duration),
            std::cmp::max(state.last_distributed, config.init_timestamp),
            std::cmp::min(cur_timestamp, last_distribution_next_timestamp),
        );
        state.current_cycle_rewards = calculate_cycle_rewards(
            state.current_cycle_rewards,
            config.reward_increase.unwrap_or(Decimal::zero()),
            state.current_cycle == last_distribution_cycle,
        );
        state.last_distributed = std::cmp::min(cur_timestamp, last_distribution_next_timestamp);
        last_distribution_cycle += 1;
    }

    if state.last_distributed == config.till_timestamp {
        state.current_cycle_rewards = Uint128::zero();
    }

    if total_share == Uint128::zero() || config.init_timestamp > cur_timestamp {
        return Ok(());
    }

    global_reward_info.index +=
        Decimal256::from(Decimal::from_ratio(rewards_to_distribute, total_share));

    Ok(())
}

fn calculate_cycles_elapsed(
    current_timestamp: u64,
    config_init_timestamp: u64,
    cycle_duration: u64,
    config_till_timestamp: u64,
) -> u64 {
    if config_init_timestamp >= current_timestamp {
        return 0u64;
    }
    let max_cycles = (config_till_timestamp - config_init_timestamp) / cycle_duration;

    let time_elapsed = current_timestamp - config_init_timestamp;
    std::cmp::min(max_cycles, time_elapsed / cycle_duration)
}

fn calculate_init_timestamp_for_cycle(
    config_init_timestamp: u64,
    current_cycle: u64,
    cycle_duration: u64,
) -> u64 {
    config_init_timestamp + (current_cycle * cycle_duration)
}

fn rewards_distributed_for_cycle(
    rewards_per_sec: Decimal,
    from_timestamp: u64,
    till_timestamp: u64,
) -> Uint128 {
    if till_timestamp <= from_timestamp {
        return Uint128::zero();
    }
    rewards_per_sec * Uint128::from(till_timestamp - from_timestamp)
}

fn calculate_cycle_rewards(
    current_cycle_rewards: Uint128,
    reward_increase_percent: Decimal,
    is_same_cycle: bool,
) -> Uint128 {
    if is_same_cycle {
        return current_cycle_rewards;
    }
    current_cycle_rewards + (current_cycle_rewards * reward_increase_percent)
}
