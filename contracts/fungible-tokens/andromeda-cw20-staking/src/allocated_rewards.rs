use andromeda_fungible_tokens::cw20_staking::{
    AllocationConfig, AllocationState, RewardToken, RewardType,
};
use andromeda_std::{
    common::{Milliseconds, MillisecondsDuration, MillisecondsExpiration},
    error::ContractError,
};

use cosmwasm_std::{BlockInfo, Decimal, Decimal256, Uint128};

/// This was taken with few changes from the MARS staking contract
/// https://github.com/mars-protocol/mars-periphery/blob/537ab8046a4670d0e80de6cbf6e6e0492c586fb2/contracts/lp_staking/src/contract.rs#L420
pub(crate) fn update_allocated_index(
    block_info: &BlockInfo,
    total_share: Uint128,
    reward_token: &mut RewardToken,
    config: AllocationConfig,
    mut state: AllocationState,
    cur_timestamp: MillisecondsExpiration,
    init_timestamp: MillisecondsExpiration,
) -> Result<(), ContractError> {
    // If the reward distribution period is over
    if state.last_distributed == config.till_timestamp.get_time(block_info)
        || !reward_token.is_active
    {
        return Ok(());
    }

    let mut last_distribution_cycle = state.current_cycle;
    state.current_cycle = calculate_cycles_elapsed(
        cur_timestamp,
        init_timestamp,
        config.cycle_duration,
        config.till_timestamp.get_time(block_info),
    );
    let mut rewards_to_distribute = Uint128::zero();
    let mut last_distribution_next_timestamp: u64; // 0 as u64;

    while state.current_cycle >= last_distribution_cycle {
        last_distribution_next_timestamp = std::cmp::min(
            config.till_timestamp.get_time(block_info).milliseconds(),
            calculate_init_timestamp_for_cycle(
                init_timestamp,
                last_distribution_cycle + 1,
                config.cycle_duration,
            )
            .milliseconds(),
        );
        rewards_to_distribute += rewards_distributed_for_cycle(
            Decimal::from_ratio(
                state.current_cycle_rewards,
                config.cycle_duration.milliseconds(),
            ),
            std::cmp::max(
                state.last_distributed.milliseconds(),
                init_timestamp.milliseconds(),
            ),
            std::cmp::min(
                cur_timestamp.milliseconds(),
                last_distribution_next_timestamp,
            ),
        );
        state.current_cycle_rewards = calculate_cycle_rewards(
            state.current_cycle_rewards,
            config.reward_increase.unwrap_or_else(Decimal::zero),
            state.current_cycle == last_distribution_cycle,
        );
        state.last_distributed = Milliseconds(std::cmp::min(
            cur_timestamp.milliseconds(),
            last_distribution_next_timestamp,
        ));

        let new_cycle = last_distribution_cycle.checked_add(1);
        match new_cycle {
            Some(new_cycle) => last_distribution_cycle = new_cycle,
            None => return Err(ContractError::Overflow {}),
        }
    }

    if state.last_distributed == config.till_timestamp.get_time(block_info) {
        state.current_cycle_rewards = Uint128::zero();
    }

    if total_share == Uint128::zero() || init_timestamp > cur_timestamp {
        return Ok(());
    }

    reward_token.index += Decimal256::from_ratio(rewards_to_distribute, total_share);
    reward_token.reward_type = RewardType::Allocated {
        allocation_config: config,
        allocation_state: state,
        init_timestamp,
    };

    Ok(())
}

fn calculate_cycles_elapsed(
    current_timestamp: MillisecondsExpiration,
    config_init_timestamp: MillisecondsExpiration,
    cycle_duration: MillisecondsDuration,
    config_till_timestamp: MillisecondsExpiration,
) -> u64 {
    if config_init_timestamp >= current_timestamp {
        return 0u64;
    }
    let max_cycles = (config_till_timestamp.minus_milliseconds(config_init_timestamp))
        .milliseconds()
        / cycle_duration.milliseconds();

    let time_elapsed = current_timestamp.minus_milliseconds(config_init_timestamp);
    std::cmp::min(
        max_cycles,
        time_elapsed.milliseconds() / cycle_duration.milliseconds(),
    )
}

fn calculate_init_timestamp_for_cycle(
    config_init_timestamp: MillisecondsExpiration,
    current_cycle: u64,
    cycle_duration: MillisecondsDuration,
) -> Milliseconds {
    config_init_timestamp
        .plus_milliseconds(Milliseconds(current_cycle * cycle_duration.milliseconds()))
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
