use andromeda_modules::string_utils::{Delimiter, GetSplitResultResponse};
use andromeda_modules::string_utils::{InstantiateMsg, QueryMsg};
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

pub fn query_split_result(
    deps: Deps,
    input: String,
    delimiter: Delimiter,
) -> Result<GetSplitResultResponse, ContractError> {
    let res = query(
        deps,
        mock_env(),
        QueryMsg::GetSplitResult { input, delimiter },
    );
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
