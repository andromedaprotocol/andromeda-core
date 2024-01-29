#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query, reply};
use andromeda_app::app::{AppComponent, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_testing::{
    mock_ado,
    mock_contract::{AnyResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};

pub struct MockApp(Addr);
mock_ado!(MockApp, ExecuteMsg, QueryMsg);

impl MockApp {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut App,
        name: impl Into<String>,
        app_components: Vec<AppComponent>,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockApp {
        let msg = mock_app_instantiate_msg(name, app_components, kernel_address, owner);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "App Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockApp(Addr::unchecked(addr))
    }

    pub fn execute_claim_ownership(
        &self,
        app: &mut App,
        sender: Addr,
        component_name: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.execute(app, &mock_claim_ownership_msg(component_name), sender, &[])
    }

    pub fn query_components(&self, app: &App) -> Vec<AppComponent> {
        self.query::<Vec<AppComponent>>(app, mock_get_components_msg())
    }

    pub fn query_component_addr(&self, app: &App, name: impl Into<String>) -> Addr {
        self.query::<Addr>(app, mock_get_address_msg(name.into()))
    }

    pub fn query_ado_by_component_name<C: From<Addr>>(
        &self,
        app: &App,
        name: impl Into<String>,
    ) -> C {
        C::from(self.query_component_addr(app, name))
    }
}

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

pub fn mock_get_address_msg(name: String) -> QueryMsg {
    QueryMsg::GetAddress { name }
}
