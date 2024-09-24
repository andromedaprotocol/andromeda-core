#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_std::amp::AndrAddr;
use andromeda_std::os::ibc_registry::{ExecuteMsg, IBCDenomInfo, InstantiateMsg};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_ibc_registry() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_ibc_registry_instantiate_msg(
    kernel_address: Addr,
    owner: Option<String>,
    service_address: AndrAddr,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        service_address,
    }
}

pub fn mock_execute_store_denom_info_msg(ibc_denom_info: Vec<IBCDenomInfo>) -> ExecuteMsg {
    ExecuteMsg::StoreDenomInfo { ibc_denom_info }
}
