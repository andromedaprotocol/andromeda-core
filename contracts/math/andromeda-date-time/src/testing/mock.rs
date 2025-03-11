use andromeda_math::date_time::{GetDateTimeResponse, Timezone};
use andromeda_math::date_time::{InstantiateMsg, QueryMsg};
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

pub fn proper_initialization() -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let creator = deps.api.addr_make("creator");
    let info = message_info(&creator, &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn query_date_time(
    deps: Deps,
    timezone: Option<Timezone>,
) -> Result<GetDateTimeResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetDateTime { timezone });
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
