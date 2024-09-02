use crate::contract::{execute, instantiate, query};
use andromeda_data_storage::boolean::{
    BooleanRestriction, ExecuteMsg, GetValueResponse, InstantiateMsg, QueryMsg,
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

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(restriction: BooleanRestriction) -> (MockDeps, MessageInfo) {
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

pub fn query_value(deps: Deps) -> Result<GetValueResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetValue {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn set_value(deps: DepsMut<'_>, value: &bool, sender: &str) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetValue { value: *value };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn set_value_with_funds(
    deps: DepsMut<'_>,
    value: &bool,
    sender: &str,
    coin: Coin,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetValue { value: *value };
    let info = mock_info(sender, &[coin]);
    execute(deps, mock_env(), info, msg)
}

pub fn delete_value(deps: DepsMut<'_>, sender: &str) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::DeleteValue {};
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}
