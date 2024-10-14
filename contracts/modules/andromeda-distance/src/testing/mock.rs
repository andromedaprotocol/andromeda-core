use andromeda_modules::distance::Coordinate;
use andromeda_modules::distance::{InstantiateMsg, QueryMsg};
use andromeda_std::{
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, MessageInfo, OwnedDeps,
};

use crate::contract::{instantiate, query};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization() -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn query_distance(
    deps: Deps,
    point_1: Coordinate,
    point_2: Coordinate,
    decimal: u16,
) -> Result<String, ContractError> {
    let res = query(
        deps,
        mock_env(),
        QueryMsg::GetDistanceBetween2Points {
            point_1,
            point_2,
            decimal,
        },
    );
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_manhattan_distance(
    deps: Deps,
    point_1: Coordinate,
    point_2: Coordinate,
    decimal: u16,
) -> Result<String, ContractError> {
    let res = query(
        deps,
        mock_env(),
        QueryMsg::GetManhattanDistance {
            point_1,
            point_2,
            decimal,
        },
    );
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
