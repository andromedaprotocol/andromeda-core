#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_socket::proxy::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::ado_base::permissioning::{Permission, PermissioningMessage};
use andromeda_std::ado_base::rates::{Rate, RatesMessage};
use andromeda_std::amp::messages::AMPPkt;
use andromeda_std::amp::AndrAddr;

use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockProxy(Addr);
mock_ado!(MockProxy, ExecuteMsg, QueryMsg);

impl MockProxy {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        admins: Vec<String>,
        kernel_address: impl Into<String>,
        owner: Option<String>,
    ) -> MockProxy {
        let msg = mock_osmosis_token_factory_instantiate_msg(admins, kernel_address, owner);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Osmosis Token Factory",
                Some(sender.to_string()),
            )
            .unwrap();
        MockProxy(Addr::unchecked(addr))
    }

    pub fn execute_add_rate(
        &self,
        app: &mut MockApp,
        sender: Addr,
        action: String,
        rate: Rate,
    ) -> ExecuteResult {
        self.execute(app, &mock_set_rate_msg(action, rate), sender, &[])
    }

    pub fn execute_set_permission(
        &self,
        app: &mut MockApp,
        sender: Addr,
        actors: Vec<AndrAddr>,
        action: String,
        permission: Permission,
    ) -> ExecuteResult {
        let msg = mock_set_permission(actors, action, permission);
        self.execute(app, &msg, sender, &[])
    }
}

pub fn mock_andromeda_osmosis_token_factory() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_osmosis_token_factory_instantiate_msg(
    admins: Vec<String>,
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        admins,
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_set_rate_msg(action: String, rate: Rate) -> ExecuteMsg {
    ExecuteMsg::Rates(RatesMessage::SetRate { action, rate })
}

pub fn mock_set_permission(
    actors: Vec<AndrAddr>,
    action: String,
    permission: Permission,
) -> ExecuteMsg {
    ExecuteMsg::Permissioning(PermissioningMessage::SetPermission {
        actors,
        action,
        permission,
    })
}

pub fn mock_receive_packet(packet: AMPPkt) -> ExecuteMsg {
    ExecuteMsg::AMPReceive(packet)
}
