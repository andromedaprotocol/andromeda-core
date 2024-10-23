#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_data_storage::point::{
    ExecuteMsg, GetDataOwnerResponse, InstantiateMsg, PointCoordinate, PointRestriction, QueryMsg,
};
use andromeda_std::ado_base::rates::{Rate, RatesMessage};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockPoint(Addr);
mock_ado!(MockPoint, ExecuteMsg, QueryMsg);

impl MockPoint {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        restriction: PointRestriction,
    ) -> MockPoint {
        let msg = mock_point_instantiate_msg(kernel_address, owner, restriction);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Point Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockPoint(Addr::unchecked(addr))
    }

    pub fn execute_set_point(
        &self,
        app: &mut MockApp,
        sender: Addr,
        point: PointCoordinate,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_point_set_point_msg(point);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
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

    pub fn query_point(&self, app: &mut MockApp) -> PointCoordinate {
        let msg = mock_point_get_point();
        let res: PointCoordinate = self.query(app, msg);
        res
    }

    pub fn query_data_owner(&self, app: &mut MockApp) -> GetDataOwnerResponse {
        let msg = mock_point_get_data_owner();
        let res: GetDataOwnerResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_point() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_point_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    restriction: PointRestriction,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        restriction,
    }
}

/// Used to generate a message to set point
pub fn mock_point_set_point_msg(point: PointCoordinate) -> ExecuteMsg {
    ExecuteMsg::SetPoint { point }
}

pub fn mock_set_rate_msg(action: String, rate: Rate) -> ExecuteMsg {
    ExecuteMsg::Rates(RatesMessage::SetRate { action, rate })
}

pub fn mock_point_get_point() -> QueryMsg {
    QueryMsg::GetPoint {}
}

pub fn mock_point_get_data_owner() -> QueryMsg {
    QueryMsg::GetDataOwner {}
}
