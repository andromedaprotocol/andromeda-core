use andromeda_data_storage::matrix::{
    ExecuteMsg, GetMatrixResponse, InstantiateMsg, Matrix, MatrixRestriction, QueryMsg,
};
use andromeda_std::{
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Coin, Deps, DepsMut, MessageInfo, OwnedDeps, Response,
};

use crate::contract::{execute, instantiate, query};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(restriction: MatrixRestriction) -> (MockDeps, MessageInfo) {
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

pub fn store_matrix(
    deps: DepsMut<'_>,
    key: &Option<String>,
    data: &Matrix,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::StoreMatrix {
        key: key.clone(),
        data: data.clone(),
    };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn store_matrix_with_funds(
    deps: DepsMut<'_>,
    key: &Option<String>,
    data: &Matrix,
    sender: &str,
    coin: Coin,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::StoreMatrix {
        key: key.clone(),
        data: data.clone(),
    };
    let info = mock_info(sender, &[coin]);
    execute(deps, mock_env(), info, msg)
}

pub fn delete_matrix(
    deps: DepsMut<'_>,
    key: &Option<String>,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::DeleteMatrix { key: key.clone() };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_matrix(deps: Deps, name: &Option<String>) -> Result<GetMatrixResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetMatrix { key: name.clone() });
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
