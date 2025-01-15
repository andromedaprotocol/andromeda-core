use andromeda_math::counter::{CounterRestriction, ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_math::counter::{
    GetCurrentAmountResponse, GetDecreaseAmountResponse, GetIncreaseAmountResponse,
    GetInitialAmountResponse, GetRestrictionResponse, State,
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

pub fn proper_initialization(
    restriction: CounterRestriction,
    initial_state: State,
) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        restriction,
        initial_state,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn increment(deps: DepsMut<'_>, sender: &str) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::Increment {};
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn decrement(deps: DepsMut<'_>, sender: &str) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::Decrement {};
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn reset(deps: DepsMut<'_>, sender: &str) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::Reset {};
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn update_restriction(
    deps: DepsMut<'_>,
    restriction: CounterRestriction,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::UpdateRestriction { restriction };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn set_increase_amount(
    deps: DepsMut<'_>,
    increase_amount: u64,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetIncreaseAmount { increase_amount };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn set_decrease_amount(
    deps: DepsMut<'_>,
    decrease_amount: u64,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetDecreaseAmount { decrease_amount };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_initial_amount(deps: Deps) -> Result<GetInitialAmountResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetInitialAmount {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_current_amount(deps: Deps) -> Result<GetCurrentAmountResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetCurrentAmount {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_increase_amount(deps: Deps) -> Result<GetIncreaseAmountResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetIncreaseAmount {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_decrease_amount(deps: Deps) -> Result<GetDecreaseAmountResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetDecreaseAmount {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_restriction(deps: Deps) -> Result<GetRestrictionResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetRestriction {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
