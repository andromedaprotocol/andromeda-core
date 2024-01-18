#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_fungible_tokens::cw20_exchange::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};
use common::app::AndrAddress;
use cosmwasm_std::{Empty, Uint128};
use cw_asset::AssetInfo;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_cw20_exchange() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_cw20_exchange_instantiate_msg(token_address: String) -> InstantiateMsg {
    InstantiateMsg {
        token_address: AndrAddress::from_string(token_address),
    }
}

pub fn mock_cw20_exchange_start_sale_msg(
    asset: AssetInfo,
    exchange_rate: Uint128,
    recipient: Option<String>,
) -> Cw20HookMsg {
    Cw20HookMsg::StartSale {
        asset,
        exchange_rate,
        recipient,
    }
}

pub fn mock_cw20_exchange_hook_purchase_msg(recipient: Option<String>) -> Cw20HookMsg {
    Cw20HookMsg::Purchase { recipient }
}

pub fn mock_cw20_exchange_purchase_msg(recipient: Option<String>) -> ExecuteMsg {
    ExecuteMsg::Purchase { recipient }
}
