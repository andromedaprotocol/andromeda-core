#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_modules::address_list::{ActorPermission, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::permissioning::LocalPermission, amp::AndrAddr};
use andromeda_testing::{
    mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract,
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

pub struct MockAddressList(Addr);
mock_ado!(MockAddressList, ExecuteMsg, QueryMsg);

impl MockAddressList {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut App,
        kernel_address: impl Into<String>,
        owner: Option<String>,
        actor_permission: Option<ActorPermission>,
    ) -> MockAddressList {
        let msg = mock_address_list_instantiate_msg(kernel_address, owner, actor_permission);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Address List Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockAddressList(Addr::unchecked(addr))
    }

    pub fn execute_actor_permission(
        &self,
        app: &mut MockApp,
        sender: Addr,
        actors: Vec<AndrAddr>,
        permission: LocalPermission,
    ) -> ExecuteResult {
        self.execute(
            app,
            &mock_add_actor_permission_msg(actors, permission),
            sender,
            &[],
        )
    }
}

pub fn mock_andromeda_address_list() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_address_list_instantiate_msg(
    kernel_address: impl Into<String>,
    owner: Option<String>,
    actor_permission: Option<ActorPermission>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
        actor_permission,
    }
}

pub fn mock_add_actor_permission_msg(
    actors: Vec<AndrAddr>,
    permission: LocalPermission,
) -> ExecuteMsg {
    ExecuteMsg::PermissionActors { actors, permission }
}

pub fn mock_query_permission_msg(actor: Addr) -> QueryMsg {
    QueryMsg::ActorPermission { actor }
}
