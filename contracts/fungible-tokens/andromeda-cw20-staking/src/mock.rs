#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20_staking::{
    AllocationConfig, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, RewardTokenUnchecked,
};
use andromeda_std::{ado_base::Module, amp::AndrAddr};
use cosmwasm_std::Empty;

use cw_asset::AssetInfoUnchecked;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_cw20_staking() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_cw20_staking_instantiate_msg(
    staking_token: impl Into<String>,
    kernel_address: impl Into<String>,
    modules: Option<Vec<Module>>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        staking_token: AndrAddr::from_string(staking_token.into()),
        additional_rewards: None,
        kernel_address: kernel_address.into(),
        modules,
        owner,
    }
}

pub fn mock_cw20_staking_add_reward_tokens(
    reward_token: AssetInfoUnchecked,
    init_timestamp: u64,
    allocation_config: Option<AllocationConfig>,
) -> ExecuteMsg {
    let reward_token = RewardTokenUnchecked {
        asset_info: reward_token,
        init_timestamp,
        allocation_config,
    };
    ExecuteMsg::AddRewardToken { reward_token }
}

pub fn mock_cw20_staking_update_global_indexes(
    asset_infos: Option<Vec<AssetInfoUnchecked>>,
) -> ExecuteMsg {
    ExecuteMsg::UpdateGlobalIndexes { asset_infos }
}

pub fn mock_cw20_stake() -> Cw20HookMsg {
    Cw20HookMsg::StakeTokens {}
}

pub fn mock_cw20_get_staker(address: String) -> QueryMsg {
    QueryMsg::Staker { address }
}
