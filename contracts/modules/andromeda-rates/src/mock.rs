#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_modules::rates::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::ado_base::rates::LocalRate;
use andromeda_testing::{mock_ado, MockADO, MockContract};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub struct MockRates(Addr);
mock_ado!(MockRates, ExecuteMsg, QueryMsg);

pub fn mock_andromeda_rates() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_rates_instantiate_msg(
    action: String,
    rate: LocalRate,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
        action,
        rate,
    }
}
