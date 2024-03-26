#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query, reply};
use andromeda_app::app::{AppComponent, ExecuteMsg, InstantiateMsg, QueryMsg};

use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_app() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

pub fn mock_app_instantiate_msg(
    name: impl Into<String>,
    app_components: Vec<AppComponent>,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        app_components,
        name: name.into(),
        kernel_address: kernel_address.into(),
        owner,
        chain_info: None,
    }
}

pub fn mock_claim_ownership_msg(component_name: Option<String>) -> ExecuteMsg {
    ExecuteMsg::ClaimOwnership {
        name: component_name,
        new_owner: None,
    }
}

pub fn mock_get_components_msg() -> QueryMsg {
    QueryMsg::GetComponents {}
}

pub fn mock_get_address_msg(name: impl Into<String>) -> QueryMsg {
    QueryMsg::GetAddress { name: name.into() }
}
