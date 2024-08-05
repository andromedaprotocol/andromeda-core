#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_modules::time_gate::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_modules::time_gate::{GateAddresses, GateTime};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockTimeGate(Addr);
mock_ado!(MockTimeGate, ExecuteMsg, QueryMsg);

impl MockTimeGate {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        gate_time: GateTime,
        gate_addresses: GateAddresses,
    ) -> MockTimeGate {
        let msg = mock_time_gate_instantiate_msg(kernel_address, owner, gate_addresses, gate_time);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Time Gate Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockTimeGate(Addr::unchecked(addr))
    }

    pub fn execute_set_gate_time(
        &self,
        app: &mut MockApp,
        sender: Addr,
        gate_time: GateTime,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::SetGateTime { gate_time };
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_update_gate_addresses(
        &self,
        app: &mut MockApp,
        sender: Addr,
        gate_addresses: GateAddresses,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::UpdateGateAddresses {
            new_gate_addresses: gate_addresses,
        };
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn query_gate_time(&self, app: &mut MockApp) -> GateTime {
        let msg = QueryMsg::GetGateTime {};
        let res: GateTime = self.query(app, msg);
        res
    }

    pub fn query_gate_addresses(&self, app: &mut MockApp) -> GateAddresses {
        let msg = QueryMsg::GetGateAddresses {};
        let res: GateAddresses = self.query(app, msg);
        res
    }

    pub fn query_path(&self, app: &mut MockApp) -> Addr {
        let msg = QueryMsg::GetPathByCurrentTime {};
        let res: Addr = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_time_gate() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_time_gate_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    gate_addresses: GateAddresses,
    gate_time: GateTime,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        gate_addresses,
        gate_time,
    }
}
