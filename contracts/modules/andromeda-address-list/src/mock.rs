#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_modules::address_list::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::ado_base::permissioning::Permission;
use andromeda_testing::{mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};
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
        actor: Addr,
        permission: Permission,
    ) -> MockAddressList {
        let msg = mock_address_list_instantiate_msg(kernel_address, owner, actor, permission);
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
        app: &mut App,
        sender: Addr,
        actor: Addr,
        permission: Permission,
    ) -> ExecuteResult {
        self.execute(
            app,
            &mock_add_actor_permission_msg(actor, permission),
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
    actor: Addr,
    permission: Permission,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
        actor,
        permission,
    }
}

pub fn mock_add_actor_permission_msg(actor: Addr, permission: Permission) -> ExecuteMsg {
    ExecuteMsg::AddActorPermission { actor, permission }
}
