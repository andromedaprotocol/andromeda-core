#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20_redeem::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_std::common::{expiration::Expiry, MillisecondsDuration};
use cosmwasm_std::{Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_cw20_redeem() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_cw20_redeem_instantiate_msg(
    token_address: String,
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        token_address: AndrAddr::from_string(token_address),
        kernel_address,
        owner,
    }
}

pub fn mock_cw20_redeem_start_redemption_condition_hook_msg(
    exchange_rate: Uint128,
    recipient: Option<Recipient>,
    start_time: Option<Expiry>,
    duration: Option<MillisecondsDuration>,
) -> Cw20HookMsg {
    Cw20HookMsg::StartRedemptionCondition {
        exchange_rate,
        recipient,
        start_time,
        duration,
    }
}

pub fn mock_cw20_redeem_hook_redeem_msg() -> Cw20HookMsg {
    Cw20HookMsg::Redeem {}
}

pub fn mock_cw20_set_redemption_condition_native_msg(
    exchange_rate: Uint128,
    recipient: Option<Recipient>,
    start_time: Option<Expiry>,
    duration: Option<MillisecondsDuration>,
) -> ExecuteMsg {
    ExecuteMsg::SetRedemptionCondition {
        exchange_rate,
        recipient,
        start_time,
        duration,
    }
}

pub fn mock_cw20_redeem_cancel_redemption_condition_msg() -> ExecuteMsg {
    ExecuteMsg::CancelRedemptionCondition {}
}

pub fn mock_get_redemption_condition() -> QueryMsg {
    QueryMsg::RedemptionCondition {}
}
