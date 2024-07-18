use andromeda_modules::string_utils::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_modules::string_utils::{GetSplitResultResponse, Delimiter};
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

pub fn try_split(
    deps: DepsMut<'_>,
    input: String,
    delimiter: Delimiter,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::Split { input, delimiter };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_split_result(deps: Deps) -> Result<GetSplitResultResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetSplitResult {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
