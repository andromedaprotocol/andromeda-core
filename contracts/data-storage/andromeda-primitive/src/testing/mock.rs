use andromeda_data_storage::primitive::{
    ExecuteMsg, GetValueResponse, InstantiateMsg, Primitive, PrimitiveRestriction, QueryMsg,
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

pub fn proper_initialization(restriction: PrimitiveRestriction) -> (MockDeps, MessageInfo) {
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

pub fn query_value(deps: Deps, name: &Option<String>) -> Result<GetValueResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetValue { key: name.clone() });
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn set_value(
    deps: DepsMut<'_>,
    key: &Option<String>,
    value: &Primitive,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetValue {
        key: key.clone(),
        value: value.clone(),
    };
    let info = message_info(&Addr::unchecked(sender), &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn set_value_with_funds(
    deps: DepsMut<'_>,
    key: &Option<String>,
    value: &Primitive,
    sender: &str,
    coin: Coin,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetValue {
        key: key.clone(),
        value: value.clone(),
    };
    let info = message_info(&Addr::unchecked(sender), &[coin]);
    execute(deps, mock_env(), info, msg)
}

pub fn delete_value(
    deps: DepsMut<'_>,
    key: &Option<String>,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::DeleteValue { key: key.clone() };
    let info = message_info(&Addr::unchecked(sender), &[]);
    execute(deps, mock_env(), info, msg)
}
