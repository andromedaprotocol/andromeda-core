use andromeda_modules::vdf_mint::{
    ExecuteMsg, GetActorsResponse, GetLastMintTimestampSecondsResponse,
    GetMintCooldownMinutesResponse, InstantiateMsg, QueryMsg,
};
use andromeda_std::{
    amp::AndrAddr, error::ContractError, testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, DepsMut, Env, MessageInfo, OwnedDeps, Response, Uint64,
};

use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn proper_initialization(
    cw721_address: AndrAddr,
    actors: Option<Vec<AndrAddr>>,
    mint_cooldown_minutes: Option<Uint64>,
) -> (MockDeps, MessageInfo) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        cw721_address,
        actors,
        mint_cooldown_minutes,
    };
    let env = mock_env();
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
    (deps, info)
}

pub fn add_actors(
    deps: DepsMut<'_>,
    actors: Vec<AndrAddr>,
    sender: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::AddActors { actors };
    let info = mock_info(sender, &[]);
    execute(deps, mock_env(), info, msg)
}

pub fn vdf_mint(
    deps: DepsMut<'_>,
    token_id: String,
    owner: AndrAddr,
    sender: &str,
    env: Env,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::VdfMint { token_id, owner };
    let info = mock_info(sender, &[]);
    execute(deps, env, info, msg)
}

pub fn query_actors(deps: Deps) -> Result<GetActorsResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetActors {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_last_mint_timestamp_seconds(
    deps: Deps,
) -> Result<GetLastMintTimestampSecondsResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetLastMintTimestampSeconds {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}

pub fn query_mint_cooldown_minutes(
    deps: Deps,
) -> Result<GetMintCooldownMinutesResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetMintCooldownMinutes {});
    match res {
        Ok(res) => Ok(from_json(res).unwrap()),
        Err(err) => Err(err),
    }
}
