use super::mock_querier::{
    mock_dependencies_custom, mock_mint_config_response, mock_staking_config_response,
    MOCK_MIRROR_GOV_ADDR, MOCK_MIRROR_MINT_ADDR, MOCK_MIRROR_STAKING_ADDR,
};
use crate::contract::{execute, instantiate, query};
use andromeda_protocol::mirror_wrapped_cdp::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MirrorGovExecuteMsg, MirrorGovQueryMsg,
    MirrorMintExecuteMsg, MirrorMintQueryMsg, MirrorStakingExecuteMsg, MirrorStakingQueryMsg,
    QueryMsg,
};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    from_binary, to_binary, CosmosMsg, Decimal, DepsMut, MessageInfo, Response, Uint128, WasmMsg,
};
use mirror_protocol::gov::{ConfigResponse as GovConfigResponse, VoteOption};
use mirror_protocol::mint::ConfigResponse as MintConfigResponse;
use mirror_protocol::staking::ConfigResponse as StakingConfigResponse;
use terraswap::asset::{Asset, AssetInfo};

fn assert_mint_execute_msg(deps: DepsMut, info: MessageInfo, mirror_msg: MirrorMintExecuteMsg) {
    let msg = ExecuteMsg::MirrorMintExecuteMsg(mirror_msg.clone());
    let res = execute(deps, mock_env(), info.clone(), msg.clone()).unwrap();

    let execute_msg = WasmMsg::Execute {
        contract_addr: MOCK_MIRROR_MINT_ADDR.to_string(),
        funds: info.funds,
        msg: to_binary(&mirror_msg).unwrap(),
    };
    assert_eq!(
        Response::new().add_messages(vec![CosmosMsg::Wasm(execute_msg)]),
        res
    );
}

fn assert_staking_execute_msg(
    deps: DepsMut,
    info: MessageInfo,
    mirror_msg: MirrorStakingExecuteMsg,
) {
    let msg = ExecuteMsg::MirrorStakingExecuteMsg(mirror_msg.clone());
    let res = execute(deps, mock_env(), info.clone(), msg.clone()).unwrap();

    let execute_msg = WasmMsg::Execute {
        contract_addr: MOCK_MIRROR_STAKING_ADDR.to_string(),
        funds: info.funds,
        msg: to_binary(&mirror_msg).unwrap(),
    };
    assert_eq!(
        Response::new().add_messages(vec![CosmosMsg::Wasm(execute_msg)]),
        res
    );
}

fn assert_gov_execute_msg(deps: DepsMut, info: MessageInfo, mirror_msg: MirrorGovExecuteMsg) {
    let msg = ExecuteMsg::MirrorGovExecuteMsg(mirror_msg.clone());
    let res = execute(deps, mock_env(), info.clone(), msg.clone()).unwrap();

    let execute_msg = WasmMsg::Execute {
        contract_addr: MOCK_MIRROR_GOV_ADDR.to_string(),
        funds: info.funds,
        msg: to_binary(&mirror_msg).unwrap(),
    };
    assert_eq!(
        Response::new().add_messages(vec![CosmosMsg::Wasm(execute_msg)]),
        res
    );
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);

    let msg = InstantiateMsg {
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("owner", info.sender),
        res
    );

    // Verify that we can query the mirror mint contract.
    let msg = QueryMsg::MirrorMintQueryMsg(MirrorMintQueryMsg::Config {});
    let res: MintConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(mock_mint_config_response(), res);

    // Verify that we can query the mirror staking contract.
    let msg = QueryMsg::MirrorStakingQueryMsg(MirrorStakingQueryMsg::Config {});
    let res: StakingConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(mock_staking_config_response(), res);

    // Verify that we can query the mirror gov contract.
    let msg = QueryMsg::MirrorGovQueryMsg(MirrorGovQueryMsg::Config {});
    // Can't check equality for this one as GovConfigResponse doesn't derive Debug for some reason.
    // But unwrapping is enough to check that it was returned.
    let _res: GovConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    // Verify that we can query our contract's config.
    let msg = QueryMsg::Config {};
    let res: ConfigResponse = from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        ConfigResponse {
            mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
            mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
            mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string()
        },
        res
    );
}

#[test]
fn test_mirror_mint_open_position() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorMintExecuteMsg::OpenPosition {
        collateral: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(10_u128),
        },
        asset_info: AssetInfo::Token {
            contract_addr: "token_address".to_string(),
        },
        collateral_ratio: Decimal::one(),
        short_params: None,
    };
    assert_mint_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_mint_deposit() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorMintExecuteMsg::Deposit {
        collateral: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(10_u128),
        },
        position_idx: Uint128::from(1_u128),
    };

    assert_mint_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_mint_withdraw() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorMintExecuteMsg::Withdraw {
        position_idx: Uint128::from(1_u128),
        collateral: None,
    };

    assert_mint_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_mint_mint() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorMintExecuteMsg::Mint {
        asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(10_u128),
        },
        position_idx: Uint128::from(1_u128),
        short_params: None,
    };

    assert_mint_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_staking_unbond() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorStakingExecuteMsg::Unbond {
        asset_token: "asset_token".to_string(),
        amount: Uint128::from(1_u128),
    };

    assert_staking_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_staking_withdraw() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorStakingExecuteMsg::Withdraw { asset_token: None };

    assert_staking_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_staking_autostake() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorStakingExecuteMsg::AutoStake {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(10_u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(10_u128),
            },
        ],
        slippage_tolerance: None,
    };

    assert_staking_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_gov_castvote() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorGovExecuteMsg::CastVote {
        poll_id: 1_u64,
        amount: Uint128::from(1_u128),
        vote: VoteOption::Yes,
    };

    assert_gov_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_gov_withdraw_voting_tokens() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorGovExecuteMsg::WithdrawVotingTokens { amount: None };

    assert_gov_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_gov_withdraw_voting_rewards() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorGovExecuteMsg::WithdrawVotingRewards { poll_id: None };

    assert_gov_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_gov_stake_voting_rewards() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorGovExecuteMsg::StakeVotingRewards { poll_id: None };

    assert_gov_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_gov_end_poll() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorGovExecuteMsg::EndPoll { poll_id: 1_u64 };

    assert_gov_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_gov_execute_poll() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorGovExecuteMsg::ExecutePoll { poll_id: 1_u64 };

    assert_gov_execute_msg(deps.as_mut(), info, mirror_msg);
}

#[test]
fn test_mirror_gov_snapshot_poll() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        mirror_gov_contract: MOCK_MIRROR_GOV_ADDR.to_string(),
        mirror_mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        mirror_staking_contract: MOCK_MIRROR_STAKING_ADDR.to_string(),
    };
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let mirror_msg = MirrorGovExecuteMsg::SnapshotPoll { poll_id: 1_u64 };

    assert_gov_execute_msg(deps.as_mut(), info, mirror_msg);
}
