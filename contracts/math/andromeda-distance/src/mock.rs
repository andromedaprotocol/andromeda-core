#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_modules::distance::{Coordinate, InstantiateMsg, QueryMsg};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockDistance(Addr);
mock_ado!(MockDistance, ExecuteMsg, QueryMsg);

impl MockDistance {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
    ) -> MockDistance {
        let msg = mock_distance_instantiate_msg(kernel_address, owner);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Distance Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockDistance(Addr::unchecked(addr))
    }

    pub fn query_distance(
        &self,
        app: &mut MockApp,
        point_1: Coordinate,
        point_2: Coordinate,
        decimal: u16,
    ) -> String {
        let msg = QueryMsg::GetDistanceBetween2Points {
            point_1,
            point_2,
            decimal,
        };
        let res: String = self.query(app, msg);
        res
    }

    pub fn query_manhattan_distance(
        &self,
        app: &mut MockApp,
        point_1: Coordinate,
        point_2: Coordinate,
        decimal: u16,
    ) -> String {
        let msg = QueryMsg::GetManhattanDistance {
            point_1,
            point_2,
            decimal,
        };
        let res: String = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_distance() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_distance_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
    }
}
