#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_non_fungible_tokens::marketplace::{Cw721HookMsg, ExecuteMsg, InstantiateMsg};
use andromeda_std::ado_base::modules::Module;
use andromeda_std::amp::messages::AMPPkt;
use cosmwasm_std::{Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_marketplace() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_marketplace_instantiate_msg(
    kernel_address: String,
    modules: Option<Vec<Module>>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        modules,
        kernel_address,
        owner,
    }
}

pub fn mock_start_sale(
    price: Uint128,
    coin_denom: impl Into<String>,
    uses_cw20: bool,
) -> Cw721HookMsg {
    Cw721HookMsg::StartSale {
        price,
        coin_denom: coin_denom.into(),
        start_time: None,
        duration: None,
        uses_cw20,
    }
}

pub fn mock_buy_token(token_address: impl Into<String>, token_id: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::Buy {
        token_id: token_id.into(),
        token_address: token_address.into(),
    }
}

pub fn mock_receive_packet(packet: AMPPkt) -> ExecuteMsg {
    ExecuteMsg::AMPReceive(packet)
}
