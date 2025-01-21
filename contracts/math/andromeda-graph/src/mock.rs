#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_math::graph::CoordinateInfo;
use andromeda_math::graph::{
    Coordinate, GetAllPointsResponse, GetMapInfoResponse, GetMaxPointNumberResponse, MapInfo,
};
use andromeda_math::graph::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockGraph(Addr);
mock_ado!(MockGraph, ExecuteMsg, QueryMsg);

impl MockGraph {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        map_info: MapInfo,
    ) -> MockGraph {
        let msg = mock_graph_instantiate_msg(kernel_address, owner, map_info);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Graph Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockGraph(Addr::unchecked(addr))
    }

    pub fn execute_update_map(
        &self,
        app: &mut MockApp,
        sender: Addr,
        map_info: MapInfo,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_update_map_msg(map_info);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_store_coordinate(
        &self,
        app: &mut MockApp,
        sender: Addr,
        coordinate: Coordinate,
        is_timestamp_allowed: bool,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::StoreCoordinate {
            coordinate,
            is_timestamp_allowed,
        };
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_store_user_coordinate(
        &self,
        app: &mut MockApp,
        sender: Addr,
        user_location_paths: Vec<AndrAddr>,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::StoreUserCoordinate {
            user_location_paths,
        };
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_delete_user_coordinate(
        &self,
        app: &mut MockApp,
        sender: Addr,
        user: AndrAddr,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::DeleteUserCoordinate { user };
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn query_map_info(&self, app: &mut MockApp) -> GetMapInfoResponse {
        let msg = QueryMsg::GetMapInfo {};
        let res: GetMapInfoResponse = self.query(app, msg);
        res
    }

    pub fn query_max_point_number(&self, app: &mut MockApp) -> GetMaxPointNumberResponse {
        let msg = QueryMsg::GetMaxPointNumber {};
        let res: GetMaxPointNumberResponse = self.query(app, msg);
        res
    }

    pub fn query_all_points(
        &self,
        app: &mut MockApp,
        start: Option<u128>,
        limit: Option<u32>,
    ) -> GetAllPointsResponse {
        let msg = QueryMsg::GetAllPoints { start, limit };
        let res: GetAllPointsResponse = self.query(app, msg);
        res
    }

    pub fn query_user_coordinate(&self, app: &mut MockApp, user: AndrAddr) -> CoordinateInfo {
        let msg = QueryMsg::GetUserCoordinate { user };
        let res: CoordinateInfo = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_graph() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_graph_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    map_info: MapInfo,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        map_info,
    }
}

pub fn mock_execute_update_map_msg(map_info: MapInfo) -> ExecuteMsg {
    ExecuteMsg::UpdateMap { map_info }
}
