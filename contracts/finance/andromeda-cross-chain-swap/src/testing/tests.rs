use andromeda_std::{
    ado_base::modules::Module, amp::AndrAddr, error::ContractError,
    testing::mock_querier::MOCK_OSMOSIS_ROUTER_CONTRACT,
};

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, wasm_execute, CosmosMsg, Decimal, DepsMut, Response, StdError, SubMsg, WasmMsg,
};

pub const OWNER: &str = "creator";

use super::mock_querier::MOCK_KERNEL_CONTRACT;

use crate::{
    contract::{execute, instantiate},
    state::{ForwardReplyState, FORWARD_REPLY_STATE},
    testing::mock_querier::mock_dependencies_custom,
};
use andromeda_finance::cross_chain_swap::{
    ExecuteMsg, InstantiateMsg, OsmosisSlippage, OsmosisSwapMsg,
};
use cosmwasm_std::coin;

fn init(deps: DepsMut) -> Response {
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_owned()),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut());
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_swap_and_forward_invalid_dex() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let in_coin = coin(100, "uosmo");
    let info = mock_info("sender", &[in_coin.clone()]);
    let env = mock_env();
    let recipient_addr = AndrAddr::from_string("recipient");
    let slippage_percentage = Decimal::percent(1);

    let msg = ExecuteMsg::SwapAndForward {
        dex: "notadex".to_string(),
        to_denom: "uusd".to_string(),
        forward_addr: recipient_addr.clone(),
        forward_msg: None,
        slippage_percentage: slippage_percentage.clone(),
        window_seconds: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(
        res,
        ContractError::Std(StdError::GenericErr {
            msg: "Unsupported Dex".to_string()
        })
    );
}

#[test]
fn test_swap_and_forward_current_state_failure() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let in_coin = coin(100, "uosmo");
    let info = mock_info("sender", &[in_coin.clone()]);
    let env = mock_env();
    let recipient_addr = AndrAddr::from_string("recipient");
    let slippage_percentage = Decimal::percent(1);

    let fake_reply_state = ForwardReplyState {
        amp_ctx: None,
        addr: recipient_addr.clone(),
        msg: None,
        dex: "osmo".to_string(),
    };
    FORWARD_REPLY_STATE
        .save(deps.as_mut().storage, &fake_reply_state)
        .unwrap();

    let msg = ExecuteMsg::SwapAndForward {
        dex: "notadex".to_string(),
        to_denom: "uusd".to_string(),
        forward_addr: recipient_addr.clone(),
        forward_msg: None,
        slippage_percentage: slippage_percentage.clone(),
        window_seconds: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(res, ContractError::Unauthorized {});
}

#[test]
fn test_swap_and_forward_osmo() {
    let mut deps = mock_dependencies_custom(&[]);
    init(deps.as_mut());
    let in_coin = coin(100, "uosmo");
    let info = mock_info("sender", &[in_coin.clone()]);
    let env = mock_env();
    let recipient_addr = AndrAddr::from_string("recipient");
    let slippage_percentage = Decimal::percent(1);

    let msg = ExecuteMsg::SwapAndForward {
        dex: "osmo".to_string(),
        to_denom: "uusd".to_string(),
        forward_addr: recipient_addr.clone(),
        forward_msg: None,
        slippage_percentage: slippage_percentage.clone(),
        window_seconds: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert!(res.messages.len() == 1);

    let expected_msg = OsmosisSwapMsg::Swap {
        input_coin: in_coin.clone(),
        output_denom: "uusd".to_string(),
        slippage: OsmosisSlippage::Twap {
            window_seconds: None,
            slippage_percentage,
        },
    };
    let expected = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_OSMOSIS_ROUTER_CONTRACT.to_string(),
        msg: to_binary(&expected_msg).unwrap(),
        funds: vec![in_coin],
    });
    assert_eq!(res.messages[0].msg, expected);

    let state = FORWARD_REPLY_STATE.load(deps.as_ref().storage).unwrap();
    let expected = ForwardReplyState {
        amp_ctx: None,
        addr: recipient_addr,
        msg: None,
        dex: "osmo".to_string(),
    };
    assert_eq!(state, expected);
}
