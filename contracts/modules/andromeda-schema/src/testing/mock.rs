use andromeda_modules::schema::{
    ExecuteMsg, GetSchemaResponse, InstantiateMsg, QueryMsg, ValidateDataResponse,
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

pub fn proper_initialization(schema_json_string: String) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        schema_json_string,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn update_schema(
    deps: DepsMut<'_>,
    sender: &str,
    new_schema_json_string: String,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::UpdateSchema {
        new_schema_json_string,
    };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_validate_data(
    deps: Deps,
    data: String,
) -> Result<ValidateDataResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::ValidateData { data });
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_schema(deps: Deps) -> Result<GetSchemaResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetSchema {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
