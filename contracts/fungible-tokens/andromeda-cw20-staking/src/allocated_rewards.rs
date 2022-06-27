use andromeda_fungible_tokens::cw20_staking::{
    AllocationConfig, AllocationState, RewardToken, RewardType,
};
use common::error::ContractError;
use cosmwasm_std::{Decimal, Decimal256, Uint128};

/// This was taken with few changes from the MARS staking contract
/// https://github.com/mars-protocol/mars-periphery/blob/537ab8046a4670d0e80de6cbf6e6e0492c586fb2/contracts/lp_staking/src/contract.rs#L420
pub(crate) fn update_allocated_index(
    total_share: Uint128,
    reward_token: &mut RewardToken,
    config: AllocationConfig,
    mut state: AllocationState,
    cur_timestamp: u64,
) -> Result<(), ContractError> {
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
            config.reward_increase.unwrap_or_else(Decimal::zero),
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

    reward_token.index += Decimal256::from_ratio(rewards_to_distribute, total_share);
    reward_token.reward_type = RewardType::Allocated {
        allocation_config: config,
        allocation_state: state,
    };

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
