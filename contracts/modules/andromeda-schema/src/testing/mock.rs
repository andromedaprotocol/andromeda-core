use andromeda_modules::schema::{
    GetSchemaResponse, InstantiateMsg, QueryMsg, ValidateDataResponse,
};
use andromeda_std::{
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    from_json,
    testing::{message_info, mock_env, MockApi, MockStorage},
    Deps, MessageInfo, OwnedDeps,
};

use crate::contract::{instantiate, query};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(schema_json_string: String) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
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
