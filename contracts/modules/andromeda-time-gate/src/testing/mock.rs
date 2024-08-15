use andromeda_modules::time_gate::CycleStartTime;
use andromeda_modules::time_gate::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    amp::AndrAddr,
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Addr, Deps, DepsMut, Env, MessageInfo, OwnedDeps, Response,
};

use crate::contract::{execute, instantiate, query};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(
    gate_addresses: Vec<AndrAddr>,
    cycle_start_time: CycleStartTime,
    time_interval: Option<u64>,
) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        gate_addresses,
        cycle_start_time,
        time_interval,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn update_cycle_start_time(
    deps: DepsMut<'_>,
    cycle_start_time: CycleStartTime,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::UpdateCycleStartTime { cycle_start_time };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn update_gate_addresses(
    deps: DepsMut<'_>,
    gate_addresses: Vec<AndrAddr>,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::UpdateGateAddresses {
        new_gate_addresses: gate_addresses,
    };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn update_time_interval(
    deps: DepsMut<'_>,
    time_interval: u64,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::UpdateTimeInterval { time_interval };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_cycle_start_time(deps: Deps) -> Result<CycleStartTime, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetCycleStartTime {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_gate_addresses(deps: Deps) -> Result<Vec<AndrAddr>, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetGateAddresses {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_time_interval(deps: Deps) -> Result<String, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetTimeInterval {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_current_ado_path(deps: Deps, env: Env) -> Result<Addr, ContractError> {
    let res = query(deps, env, QueryMsg::GetCurrentAdoPath {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
