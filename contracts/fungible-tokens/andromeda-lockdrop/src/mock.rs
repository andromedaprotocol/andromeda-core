#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::lockdrop::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};
use andromeda_std::{ado_base::modules::Module, amp::AndrAddr, common::Milliseconds};
use cosmwasm_std::{Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_lockdrop() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

#[allow(clippy::too_many_arguments)]
pub fn mock_lockdrop_instantiate_msg(
    init_timestamp: Milliseconds,
    deposit_window: Milliseconds,
    withdrawal_window: Milliseconds,
    incentive_token: AndrAddr,
    native_denom: String,
    owner: Option<String>,
    modules: Option<Vec<Module>>,
    kernel_address: String,
) -> InstantiateMsg {
    InstantiateMsg {
        init_timestamp,
        deposit_window,
        withdrawal_window,
        native_denom,
        incentive_token,
        modules,
        kernel_address,
        owner,
    }
}

pub fn mock_deposit_native() -> ExecuteMsg {
    ExecuteMsg::DepositNative {}
}

pub fn mock_enable_claims() -> ExecuteMsg {
    ExecuteMsg::EnableClaims {}
}

pub fn mock_claim_rewards() -> ExecuteMsg {
    ExecuteMsg::ClaimRewards {}
}

pub fn mock_withdraw_native(amount: Option<Uint128>) -> ExecuteMsg {
    ExecuteMsg::WithdrawNative { amount }
}

pub fn mock_cw20_hook_increase_incentives() -> Cw20HookMsg {
    Cw20HookMsg::IncreaseIncentives {}
}
