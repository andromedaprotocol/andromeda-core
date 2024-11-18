use andromeda_data_storage::graph::{Coordinate, GetMapInfoResponse, MapInfo};
use andromeda_data_storage::graph::{
    CoordinateInfo, ExecuteMsg, GetAllPointsResponse, GetMaxPointNumberResponse, InstantiateMsg,
    QueryMsg,
};
use andromeda_std::{
    amp::AndrAddr, error::ContractError, testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, DepsMut, MessageInfo, OwnedDeps, Response,
};

use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(map_info: MapInfo) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("sender", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        map_info,
    };
    let env = mock_env();
    instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
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
    is_timestamp_allowed: bool,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::StoreCoordinate {
        coordinate,
        is_timestamp_allowed,
    };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn store_user_coordinate(
    deps: DepsMut<'_>,
    user_location_paths: Vec<AndrAddr>,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::StoreUserCoordinate {
        user_location_paths,
    };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn delete_user_coordinate(
    deps: DepsMut<'_>,
    user: AndrAddr,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::DeleteUserCoordinate { user };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_map_info(deps: Deps) -> Result<GetMapInfoResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetMapInfo {});
    match res {
        Ok(res) => Ok(from_json(res)?),
        Err(err) => Err(err),
    }
}

pub fn query_max_point_number(deps: Deps) -> Result<GetMaxPointNumberResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetMaxPointNumber {});
    match res {
        Ok(res) => Ok(from_json(res)?),
        Err(err) => Err(err),
    }
}

pub fn query_all_points(
    deps: Deps,
    start: Option<u128>,
    limit: Option<u32>,
) -> Result<GetAllPointsResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetAllPoints { start, limit });
    match res {
        Ok(res) => Ok(from_json(res)?),
        Err(err) => Err(err),
    }
}

pub fn query_user_coordinate(deps: Deps, user: AndrAddr) -> Result<CoordinateInfo, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetUserCoordinate { user });
    match res {
        Ok(res) => Ok(from_json(res)?),
        Err(err) => Err(err),
    }
}
