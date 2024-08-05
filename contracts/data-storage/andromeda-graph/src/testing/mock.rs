use andromeda_data_storage::graph::{Coordinate, GetMapInfoResponse, MapInfo};
use andromeda_data_storage::graph::{
    ExecuteMsg, GetAllPointsResponse, GetMaxPointResponse, InstantiateMsg, QueryMsg,
};
use andromeda_std::{
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, DepsMut, MessageInfo, OwnedDeps, Response,
};

use crate::contract::{execute, instantiate, query};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(map_info: MapInfo) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        map_info,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn update_map(
    deps: DepsMut<'_>,
    map_info: MapInfo,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::UpdateMap { map_info };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn store_coordinate(
    deps: DepsMut<'_>,
    coordinate: Coordinate,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::StoreCoordinate { coordinate };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_map_info(deps: Deps) -> Result<GetMapInfoResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetMapInfo {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_max_point(deps: Deps) -> Result<GetMaxPointResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetMaxPoint {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_all_points(deps: Deps) -> Result<GetAllPointsResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetAllPoints {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
