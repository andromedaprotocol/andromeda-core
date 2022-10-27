#![cfg(not(target_arch = "wasm32"))]

use crate::contract::{execute, instantiate, query};
use andromeda_modules::rates::{InstantiateMsg, RateInfo};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_rates() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_rates_instantiate_msg(rates: Vec<RateInfo>) -> InstantiateMsg {
    InstantiateMsg { rates }
}
