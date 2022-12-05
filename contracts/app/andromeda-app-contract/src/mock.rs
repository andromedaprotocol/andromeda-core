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
    name: String,
    app_components: Vec<AppComponent>,
    primitive_contract: String,
) -> InstantiateMsg {
    InstantiateMsg {
        app_components,
        name,
        primitive_contract,
        target_ados: None,
    }
}

pub fn mock_claim_ownership_msg(component_name: Option<String>) -> ExecuteMsg {
    ExecuteMsg::ClaimOwnership {
        name: component_name,
    }
}

pub fn mock_get_components_msg() -> QueryMsg {
    QueryMsg::GetComponents {}
}

pub fn mock_get_address_msg(name: String) -> QueryMsg {
    QueryMsg::GetAddress { name }
}
