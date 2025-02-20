use andromeda_std::{
    ado_base::InstantiateMsg,
    ado_contract::ADOContract,
    amp::messages::{AMPMsg, AMPPkt},
    andr_exec, andr_execute_fn,
    common::context::ExecuteContext,
    error::ContractError,
    testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier, MOCK_KERNEL_CONTRACT},
    unwrap_amp_msg,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    to_json_binary, Env, MessageInfo, OwnedDeps, QuerierWrapper, Response,
};
use cw_utils::PaymentError;
use rstest::*;

const OWNER: &str = "sender";
const ATTACKER: &str = "attacker";

#[andr_exec]
#[cw_serde]
enum ExecuteMsg {
    #[attrs(restricted, nonpayable, direct)]
    AllAttrs,
    #[attrs(restricted, nonpayable)]
    RestrictedNonPayable,
    #[attrs(restricted, direct)]
    RestrictedDirect,
    #[attrs(nonpayable, direct)]
    NonPayableDirect,
    #[attrs(restricted)]
    Restricted,
}

#[andr_execute_fn]
fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ExecuteError> {
    Ok(Response::new())
}

#[fixture]
fn setup() -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Env) {
    let mut deps = mock_dependencies_custom(&[]);
    let querier = QuerierWrapper::new(&deps.querier);
    let env = mock_env();
    let info = mock_info(OWNER, &[]);

    ADOContract::default()
        .instantiate(
            &mut deps.storage,
            env.clone(),
            &deps.api,
            &querier,
            info.clone(),
            InstantiateMsg {
                ado_type: "andromeda-test".to_string(),
                ado_version: "0.1.0".to_string(),
                owner: Some("sender".to_string()),
                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
            },
        )
        .unwrap();
    (deps, env)
}

/// Tests calling the execute entry point directly on each enum variant
/// The first test case is without funds, the second is with funds
#[rstest]
fn test_exec_fn_direct(
    setup: (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Env),
    #[values(
        // AllAttrs (restricted, nonpayable, direct)
        (ExecuteMsg::AllAttrs, OWNER, None, Some(ContractError::Payment(PaymentError::NonPayable {}))),
        (ExecuteMsg::AllAttrs, ATTACKER, Some(ContractError::Unauthorized {}), Some(ContractError::Unauthorized {})),

        // RestrictedNonPayable (restricted, nonpayable)
        (ExecuteMsg::RestrictedNonPayable, OWNER, None, Some(ContractError::Payment(PaymentError::NonPayable {}))),
        (ExecuteMsg::RestrictedNonPayable, ATTACKER, Some(ContractError::Unauthorized {}), Some(ContractError::Unauthorized {})),

        // RestrictedDirect (restricted, direct)
        (ExecuteMsg::RestrictedDirect, OWNER, None, None),
        (ExecuteMsg::RestrictedDirect, ATTACKER, Some(ContractError::Unauthorized {}), Some(ContractError::Unauthorized {})),

        // NonPayableDirect (nonpayable, direct)
        (ExecuteMsg::NonPayableDirect, OWNER, None, Some(ContractError::Payment(PaymentError::NonPayable {}))),
        (ExecuteMsg::NonPayableDirect, ATTACKER, None, Some(ContractError::Payment(PaymentError::NonPayable {})))
    )]
    case: (
        ExecuteMsg,
        &str,
        Option<ContractError>,
        Option<ContractError>,
    ),
) {
    let (mut deps, env) = setup;

    // Test case with no funds
    let send_info_no_funds = mock_info(case.1, &[]);
    let res_no_funds = execute(
        deps.as_mut(),
        env.clone(),
        send_info_no_funds,
        case.0.clone(),
    );

    match case.2 {
        Some(expected) => assert_eq!(res_no_funds, Err(expected)),
        None => assert!(res_no_funds.is_ok()),
    }

    // Test case with funds
    let send_info_with_funds = mock_info(case.1, &[coin(100, "uatom")]);
    let res_with_funds = execute(deps.as_mut(), env, send_info_with_funds, case.0);

    match case.3 {
        Some(expected) => assert_eq!(res_with_funds, Err(expected)),
        None => assert!(res_with_funds.is_ok()),
    }
}

fn direct_msg_error(msg: &str) -> Option<ContractError> {
    Some(ContractError::InvalidPacket {
        error: Some(format!("{} cannot be received via AMP packet", msg)),
    })
}

/// Tests calling the execute entry point via AMP packet on each enum variant
/// The first test case is without funds, the second is with funds
#[rstest]
fn test_exec_fn_amp_receive(
    setup: (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Env),
    #[values(
        // AllAttrs (restricted, nonpayable, direct)
        (ExecuteMsg::AllAttrs, OWNER, direct_msg_error("AllAttrs"), direct_msg_error("AllAttrs")),
        (ExecuteMsg::AllAttrs, ATTACKER, direct_msg_error("AllAttrs"), direct_msg_error("AllAttrs")),

        // RestrictedNonPayable (restricted, nonpayable)
        (ExecuteMsg::RestrictedNonPayable, OWNER, None, Some(ContractError::Payment(PaymentError::NonPayable {}))),
        (ExecuteMsg::RestrictedNonPayable, ATTACKER, Some(ContractError::Unauthorized {}), Some(ContractError::Unauthorized {})),

        // RestrictedDirect (restricted, direct)
        (ExecuteMsg::RestrictedDirect, OWNER, direct_msg_error("RestrictedDirect"), direct_msg_error("RestrictedDirect")),
        (ExecuteMsg::RestrictedDirect, ATTACKER, direct_msg_error("RestrictedDirect"), direct_msg_error("RestrictedDirect")),

        // NonPayableDirect (nonpayable, direct)
        (ExecuteMsg::NonPayableDirect, OWNER, direct_msg_error("NonPayableDirect"), direct_msg_error("NonPayableDirect")),
        (ExecuteMsg::NonPayableDirect, ATTACKER, direct_msg_error("NonPayableDirect"), direct_msg_error("NonPayableDirect"))
    )]
    case: (
        ExecuteMsg,
        &str,
        Option<ContractError>,
        Option<ContractError>,
    ),
) {
    let (mut deps, env) = setup;

    // Test case with no funds
    let send_info_no_funds = mock_info(case.1, &[]);
    let amp_msg_no_funds = AMPMsg::new(
        env.contract.address.clone(),
        to_json_binary(&case.0.clone()).unwrap(),
        None,
    );
    let amp_pkt_no_funds = AMPPkt::new(
        case.1.to_string(),
        case.1.to_string(),
        vec![amp_msg_no_funds],
    );
    let res_no_funds = execute(
        deps.as_mut(),
        env.clone(),
        send_info_no_funds,
        ExecuteMsg::AMPReceive(amp_pkt_no_funds),
    );

    match case.2 {
        Some(expected) => assert_eq!(res_no_funds, Err(expected)),
        None => assert!(res_no_funds.is_ok()),
    }

    // Test case with funds
    let send_info_with_funds = mock_info(case.1, &[coin(100, "uatom")]);
    let amp_msg_with_funds = AMPMsg::new(
        env.contract.address.clone(),
        to_json_binary(&case.0.clone()).unwrap(),
        Some(vec![coin(100, "uatom")]),
    );
    let amp_pkt_with_funds = AMPPkt::new(
        case.1.to_string(),
        case.1.to_string(),
        vec![amp_msg_with_funds],
    );
    let res_with_funds = execute(
        deps.as_mut(),
        env,
        send_info_with_funds,
        ExecuteMsg::AMPReceive(amp_pkt_with_funds),
    );

    match case.3 {
        Some(expected) => assert_eq!(res_with_funds, Err(expected)),
        None => assert!(res_with_funds.is_ok()),
    }
}

#[test]
fn test_unwrap_amp_msg() {
    let (mut deps, env) = setup();
    let info: MessageInfo = mock_info(MOCK_KERNEL_CONTRACT, &[]);

    let sent_msg = ExecuteMsg::Restricted;
    let amp_msg = AMPMsg::new(
        "sender".to_string(),
        to_json_binary(&sent_msg).unwrap(),
        None,
    );
    let amp_pkt = AMPPkt::new("sender".to_string(), "sender".to_string(), vec![amp_msg]);
    let msg = ExecuteMsg::AMPReceive(amp_pkt);

    let (ctx, msg, _) = (|| -> Result<(ExecuteContext, ExecuteMsg, Response), ContractError> {
        let (ctx, msg, resp) = unwrap_amp_msg!(deps.as_mut(), info.clone(), env.clone(), msg);
        Ok((ctx, msg, resp))
    })()
    .unwrap();

    assert_eq!(ctx.info.sender, "sender");
    assert_eq!(ctx.raw_info.sender, MOCK_KERNEL_CONTRACT);
    assert_eq!(msg, sent_msg);

    let empty_amp_pkt = AMPPkt::new("sender".to_string(), "sender".to_string(), vec![]);
    let msg = ExecuteMsg::AMPReceive(empty_amp_pkt);

    let err = (|| -> Result<(), ContractError> {
        let _ = unwrap_amp_msg!(deps.as_mut(), info, env, msg);
        Ok(())
    })()
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::InvalidPacket {
            error: Some("AMP Packet received with no messages".to_string())
        }
    );
}
