#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20_exchange::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    amp::{AndrAddr, Recipient},
    common::{denom::Asset, Schedule},
};
use cosmwasm_std::{Decimal256, Empty, Uint128};

use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_cw20_exchange() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_cw20_exchange_instantiate_msg(
    token_address: AndrAddr,
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        token_address,
        kernel_address,
        owner,
    }
}

pub fn mock_cw20_exchange_start_sale_msg(
    asset: Asset,
    exchange_rate: Uint128,
    recipient: Option<Recipient>,
    schedule: Schedule,
) -> Cw20HookMsg {
    Cw20HookMsg::StartSale {
        asset,
        exchange_rate,
        recipient,
        schedule,
    }
}

pub fn mock_cw20_exchange_hook_purchase_msg(recipient: Option<Recipient>) -> Cw20HookMsg {
    Cw20HookMsg::Purchase { recipient }
}

pub fn mock_cw20_exchange_purchase_msg(recipient: Option<Recipient>) -> ExecuteMsg {
    ExecuteMsg::Purchase { recipient }
}

pub fn mock_redeem_cw20_msg(recipient: Option<Recipient>) -> Cw20HookMsg {
    Cw20HookMsg::Redeem { recipient }
}

pub fn mock_replenish_redeem_cw20_msg(redeem_asset: Asset) -> Cw20HookMsg {
    Cw20HookMsg::ReplenishRedeem { redeem_asset }
}

pub fn mock_redeem_native_msg(recipient: Option<Recipient>) -> ExecuteMsg {
    ExecuteMsg::Redeem { recipient }
}

pub fn mock_replenish_redeem_native_msg(redeem_asset: Asset) -> ExecuteMsg {
    ExecuteMsg::ReplenishRedeem { redeem_asset }
}

pub fn mock_start_redeem_cw20_msg(
    recipient: Option<Recipient>,
    redeem_asset: Asset,
    exchange_rate: Decimal256,
    schedule: Schedule,
) -> Cw20HookMsg {
    Cw20HookMsg::StartRedeem {
        recipient,
        redeem_asset,
        exchange_rate,
        schedule,
    }
}

pub fn mock_set_redeem_condition_native_msg(
    redeem_asset: Asset,
    exchange_rate: Decimal256,
    recipient: Option<Recipient>,
    schedule: Schedule,
) -> ExecuteMsg {
    ExecuteMsg::StartRedeem {
        redeem_asset,
        exchange_rate,
        recipient,
        schedule,
    }
}

pub fn mock_redeem_query_msg(asset: Asset) -> QueryMsg {
    QueryMsg::Redeem { asset }
}
