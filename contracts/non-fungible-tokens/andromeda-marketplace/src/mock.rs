#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::marketplace::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use common::ado_base::modules::Module;
use cosmwasm_std::{Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_marketplace() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_marketplace_instantiate_msg(modules: Option<Vec<Module>>) -> InstantiateMsg {
    InstantiateMsg { modules }
}

pub fn mock_start_sale(price: Uint128, coin_denom: impl Into<String>) -> Cw721HookMsg {
    Cw721HookMsg::StartSale {
        price,
        coin_denom: coin_denom.into(),
    }
}

pub fn mock_buy_token(token_address: impl Into<String>, token_id: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::Buy {
        token_id: token_id.into(),
        token_address: token_address.into(),
    }
}
