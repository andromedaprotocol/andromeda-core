#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query, reply};
use andromeda_app::app::{AppComponent, ChainInfo, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_testing::{
    mock::MockApp,
    mock_ado,
    mock_contract::{AnyResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{AppResponse, Contract, ContractWrapper, Executor};

pub struct MockAppContract(Addr);
mock_ado!(MockAppContract, ExecuteMsg, QueryMsg);

impl MockAppContract {
    pub fn instantiate(
        code_id: u64,
        sender: &Addr,
        app: &mut MockApp,
        name: impl Into<String>,
        app_components: Vec<AppComponent>,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockAppContract {
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
        MockAppContract(Addr::unchecked(addr))
    }

    pub fn execute_claim_ownership(
        &self,
        app: &mut MockApp,
        sender: Addr,
        component_name: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.execute(app, &mock_claim_ownership_msg(component_name), sender, &[])
    }

    pub fn execute_add_app_component(
        &self,
        app: &mut MockApp,
        sender: Addr,
        component: AppComponent,
        chain_info: Option<ChainInfo>,
    ) -> AnyResult<AppResponse> {
        self.execute(
            app,
            &mock_add_app_component_msg(component, chain_info),
            sender,
            &[],
        )
    }

    pub fn query_components(&self, app: &MockApp) -> Vec<AppComponent> {
        self.query::<Vec<AppComponent>>(app, mock_get_components_msg())
    }

    pub fn query_component_addr(&self, app: &MockApp, name: impl Into<String>) -> Addr {
        self.query::<Addr>(app, mock_get_address_msg(name.into()))
    }

    pub fn query_ado_by_component_name<C: From<Addr>>(
        &self,
        app: &MockApp,
        name: impl Into<String>,
    ) -> C {
        C::from(self.query_component_addr(app, name))
    }

    pub fn query_app_address(&self, app: &MockApp) -> Option<Addr> {
        self.query::<Option<Addr>>(app, mock_get_app_address_msg())
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

pub fn mock_add_app_component_msg(
    component: AppComponent,
    chain_info: Option<ChainInfo>,
) -> ExecuteMsg {
    ExecuteMsg::AddAppComponent {
        component,
        chain_info,
    }
}

pub fn mock_get_components_msg() -> QueryMsg {
    QueryMsg::GetComponents {}
}

pub fn mock_get_adresses_with_names_msg() -> QueryMsg {
    QueryMsg::GetAddressesWithNames {}
}

pub fn mock_get_address_msg(name: impl Into<String>) -> QueryMsg {
    QueryMsg::GetAddress { name: name.into() }
}

pub fn mock_get_app_address_msg() -> QueryMsg {
    QueryMsg::AppContract {}
}
