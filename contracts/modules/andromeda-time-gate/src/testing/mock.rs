use andromeda_modules::time_gate::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_modules::time_gate::{
    GateAddresses, GateTime, 
};
use andromeda_std::{
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, DepsMut, MessageInfo, OwnedDeps, Response, Addr,
};

use crate::contract::{execute, instantiate, query};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization (
    gate_addresses: GateAddresses,
    gate_time: GateTime,
) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        gate_addresses,
        gate_time,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn set_gate_time (
    deps: DepsMut<'_>,
    gate_time: GateTime,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SetGateTime { gate_time };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn update_gate_addresses (
    deps: DepsMut<'_>,
    gate_addresses: GateAddresses,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::UpdateGateAddresses { new_gate_addresses: gate_addresses };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn query_gate_time(deps: Deps) -> Result<GateTime, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetGateTime {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_gate_addresses(deps: Deps) -> Result<GateAddresses, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetGateAddresses {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_path(deps: Deps) -> Result<Addr, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetPathByCurrentTime {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
