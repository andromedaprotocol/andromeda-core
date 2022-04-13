use crate::contract::{execute, execute_andr_receive, query};
use crate::ibc::Ics20Packet;
use crate::testing::test_helpers::{mock_channel_info, setup, DEFAULT_TIMEOUT};
use andromeda_protocol::portal_ado::{
    ChannelResponse, ExecuteMsg, ListChannelsResponse, QueryMsg, TransferMsg, WhitelistResponse,
};
use common::ado_base::{AndromedaMsg, AndromedaQuery};
use common::error::ContractError;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{coin, coins, from_binary, to_binary, CosmosMsg, IbcMsg, StdError, Uint128};
use cw0::PaymentError;
use cw20::Cw20ReceiveMsg;

#[test]
fn test_transfer() {
    let send_channel = "channel-15";
    let cw20_addr = "my-token";
    let mut deps = setup(&["channel-3", send_channel], &[cw20_addr]);

    let transfer = TransferMsg {
        channel: send_channel.to_string(),
        remote_address: "foreign-address".to_string(),
        timeout: Some(7777),
    };

    let andr_msg = AndromedaMsg::Receive(Some(to_binary(&transfer).unwrap()));
    let info = mock_info("foobar", &coins(1234567, "ucosm"));
    let res = execute_andr_receive(deps.as_mut(), mock_env(), info, andr_msg).unwrap();
    assert_eq!(res.messages[0].gas_limit, None);
    assert_eq!(1, res.messages.len());
    if let CosmosMsg::Ibc(IbcMsg::SendPacket {
        channel_id,
        data,
        timeout,
    }) = &res.messages[0].msg
    {
        let expected_timeout = mock_env().block.time.plus_seconds(7777);
        assert_eq!(timeout, &expected_timeout.into());
        assert_eq!(channel_id.as_str(), send_channel);
        let msg: Ics20Packet = from_binary(data).unwrap();
        assert_eq!(msg.amount, Uint128::new(1234567));
        assert_eq!(msg.denom.as_str(), "ucosm");
        assert_eq!(msg.sender.as_str(), "foobar");
        assert_eq!(msg.receiver.as_str(), "foreign-address");
    } else {
        panic!("Unexpected return message: {:?}", res.messages[0]);
    }
}

#[test]
fn setup_and_query() {
    let deps = setup(&["channel-3", "channel-7"], &["WHITELIST1"]);

    let raw_list = query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap();
    let list_res: ListChannelsResponse = from_binary(&raw_list).unwrap();
    assert_eq!(2, list_res.channels.len());
    assert_eq!(mock_channel_info("channel-3"), list_res.channels[0]);
    assert_eq!(mock_channel_info("channel-7"), list_res.channels[1]);

    let raw_channel = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Channel {
            id: "channel-3".to_string(),
        },
    )
    .unwrap();
    let chan_res: ChannelResponse = from_binary(&raw_channel).unwrap();
    assert_eq!(chan_res.info, mock_channel_info("channel-3"));
    assert_eq!(0, chan_res.total_sent.len());
    assert_eq!(0, chan_res.balances.len());

    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Channel {
            id: "channel-10".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::Std(StdError::NotFound {
            kind: "andromeda_protocol::portal_ado::ChannelInfo".to_string()
        })
    );

    let whitelist_query = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AndrQuery(AndromedaQuery::Get(Some(to_binary("WHITELIST1").unwrap()))),
    )
    .unwrap();
    let res: WhitelistResponse = from_binary(&whitelist_query).unwrap();
    assert_eq!(res, WhitelistResponse { is_whitelist: true })
}

#[test]
fn proper_checks_on_execute_native() {
    let send_channel = "channel-5";
    let mut deps = setup(&[send_channel, "channel-10"], &[]);

    let mut transfer = TransferMsg {
        channel: send_channel.to_string(),
        remote_address: "foreign-address".to_string(),
        timeout: None,
    };

    // works with proper funds
    let msg = ExecuteMsg::Transfer(transfer.clone());
    let info = mock_info("foobar", &coins(1234567, "ucosm"));
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.messages[0].gas_limit, None);
    assert_eq!(1, res.messages.len());
    if let CosmosMsg::Ibc(IbcMsg::SendPacket {
        channel_id,
        data,
        timeout,
    }) = &res.messages[0].msg
    {
        let expected_timeout = mock_env().block.time.plus_seconds(DEFAULT_TIMEOUT);
        assert_eq!(timeout, &expected_timeout.into());
        assert_eq!(channel_id.as_str(), send_channel);
        let msg: Ics20Packet = from_binary(data).unwrap();
        assert_eq!(msg.amount, Uint128::new(1234567));
        assert_eq!(msg.denom.as_str(), "ucosm");
        assert_eq!(msg.sender.as_str(), "foobar");
        assert_eq!(msg.receiver.as_str(), "foreign-address");
    } else {
        panic!("Unexpected return message: {:?}", res.messages[0]);
    }

    // reject with no funds
    let msg = ExecuteMsg::Transfer(transfer.clone());
    let info = mock_info("foobar", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Payment(PaymentError::NoFunds {}));

    // reject with multiple tokens funds
    let msg = ExecuteMsg::Transfer(transfer.clone());
    let info = mock_info("foobar", &[coin(1234567, "ucosm"), coin(54321, "uatom")]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Payment(PaymentError::MultipleDenoms {}));

    // reject with bad channel id
    transfer.channel = "channel-45".to_string();
    let msg = ExecuteMsg::Transfer(transfer);
    let info = mock_info("foobar", &coins(1234567, "ucosm"));
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::NoSuchChannel {
            id: "channel-45".to_string()
        }
    );
}

#[test]
fn proper_checks_on_execute_cw20() {
    let send_channel = "channel-15";
    let cw20_addr = "my-token";
    let mut deps = setup(&["channel-3", send_channel], &[cw20_addr]);

    let transfer = TransferMsg {
        channel: send_channel.to_string(),
        remote_address: "foreign-address".to_string(),
        timeout: Some(7777),
    };
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "my-account".into(),
        amount: Uint128::new(888777666),
        msg: to_binary(&transfer).unwrap(),
    });

    // works with proper funds
    let info = mock_info(cw20_addr, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
    assert_eq!(1, res.messages.len());
    assert_eq!(res.messages[0].gas_limit, None);
    if let CosmosMsg::Ibc(IbcMsg::SendPacket {
        channel_id,
        data,
        timeout,
    }) = &res.messages[0].msg
    {
        let expected_timeout = mock_env().block.time.plus_seconds(7777);
        assert_eq!(timeout, &expected_timeout.into());
        assert_eq!(channel_id.as_str(), send_channel);
        let msg: Ics20Packet = from_binary(data).unwrap();
        assert_eq!(msg.amount, Uint128::new(888777666));
        assert_eq!(msg.denom, format!("cw20:{}", cw20_addr));
        assert_eq!(msg.sender.as_str(), "my-account");
        assert_eq!(msg.receiver.as_str(), "foreign-address");
    } else {
        panic!("Unexpected return message: {:?}", res.messages[0]);
    }

    // reject with tokens funds
    let info = mock_info("foobar", &coins(1234567, "ucosm"));
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::NotOnAllowList);
}

#[test]
fn execute_cw20_fails_if_not_whitelisted() {
    let send_channel = "channel-15";
    let mut deps = setup(&["channel-3", send_channel], &[]);

    let cw20_addr = "my-token";
    let transfer = TransferMsg {
        channel: send_channel.to_string(),
        remote_address: "foreign-address".to_string(),
        timeout: Some(7777),
    };
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "my-account".into(),
        amount: Uint128::new(888777666),
        msg: to_binary(&transfer).unwrap(),
    });

    // works with proper funds
    let info = mock_info(cw20_addr, &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::NotOnAllowList);
}
