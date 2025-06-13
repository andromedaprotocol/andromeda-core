use andromeda_data_storage::string_storage::{
    ExecuteMsg, GetValueResponse, InstantiateMsg, QueryMsg, StringStorage, StringStorageRestriction,
};
use andromeda_std::{
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    from_json,
    testing::{message_info, mock_env, MockApi, MockStorage},
    Addr, Coin, Deps, DepsMut, MessageInfo, OwnedDeps, Response,
};

use crate::contract::{execute, instantiate, query};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(restriction: StringStorageRestriction) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
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

pub fn set_value(
    deps: DepsMut<'_>,
    value: &StringStorage,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetValue {
        value: value.clone(),
    };
    let info = message_info(&Addr::unchecked(sender), &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn set_value_with_funds(
    deps: DepsMut<'_>,
    value: &StringStorage,
    sender: &str,
    coin: Coin,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetValue {
        value: value.clone(),
    };
    let info = message_info(&Addr::unchecked(sender), &[coin]);
    execute(deps, mock_env(), info, msg)
}

pub fn delete_value(deps: DepsMut<'_>, sender: &str) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::DeleteValue {};
    let info = message_info(&Addr::unchecked(sender), &[]);
    execute(deps, mock_env(), info, msg)
}
