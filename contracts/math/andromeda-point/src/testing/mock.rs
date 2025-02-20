use andromeda_math::point::{
    ExecuteMsg, InstantiateMsg, PointCoordinate, PointRestriction, QueryMsg,
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

pub fn proper_initialization(restriction: PointRestriction) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        restriction,
    };
    let env = mock_env();
    instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    (deps, info)
}

pub fn query_point(deps: Deps) -> Result<PointCoordinate, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetPoint {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn set_point(
    deps: DepsMut<'_>,
    point: &PointCoordinate,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetPoint {
        point: point.clone(),
    };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn delete_point(deps: DepsMut<'_>, sender: &str) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::DeletePoint {};
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}
