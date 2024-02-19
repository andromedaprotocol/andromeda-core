use crate::{
    contract::{execute, instantiate},
    ibc::PACKET_LIFETIME,
    state::{ADO_OWNER, CHAIN_TO_CHANNEL, CHANNEL_TO_CHAIN, KERNEL_ADDRESSES},
};
use andromeda_std::{
    amp::{
        messages::{AMPMsg, AMPPkt},
        ADO_DB_KEY, VFS_KEY,
    },
    error::ContractError,
    os::kernel::{ChannelInfo, ExecuteMsg, IbcExecuteMsg, InstantiateMsg, InternalMsg},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_ADODB_CONTRACT, MOCK_FAKE_KERNEL_CONTRACT,
        MOCK_KERNEL_CONTRACT, MOCK_VFS_CONTRACT,
    },
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Binary, CosmosMsg, IbcMsg,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        owner: None,
        chain_name: "test".to_string(),
    };
    let env = mock_env();

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_create_ado() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let env = mock_env();
    instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        InstantiateMsg {
            owner: None,
            chain_name: "test".to_string(),
        },
    )
    .unwrap();

    let assign_key_msg = ExecuteMsg::UpsertKeyAddress {
        key: ADO_DB_KEY.to_string(),
        value: MOCK_ADODB_CONTRACT.to_string(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), assign_key_msg).unwrap();
    let assign_key_msg = ExecuteMsg::UpsertKeyAddress {
        key: VFS_KEY.to_string(),
        value: MOCK_VFS_CONTRACT.to_string(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), assign_key_msg).unwrap();

    let create_msg = ExecuteMsg::Create {
        ado_type: "ado_type".to_string(),
        msg: Binary::default(),
        owner: None,
        chain: None,
    };
    let res = execute(deps.as_mut(), env, info.clone(), create_msg).unwrap();
    assert_eq!(1, res.messages.len());
    assert_eq!(ADO_OWNER.load(deps.as_ref().storage).unwrap(), info.sender);
}

#[test]
fn test_register_user_cross_chain() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let env = mock_env();
    let chain = "chain";
    instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        InstantiateMsg {
            owner: None,
            chain_name: "andromeda".to_string(),
        },
    )
    .unwrap();

    KERNEL_ADDRESSES
        .save(
            deps.as_mut().storage,
            VFS_KEY,
            &Addr::unchecked(MOCK_VFS_CONTRACT),
        )
        .unwrap();
    let channel_info = ChannelInfo {
        kernel_address: MOCK_FAKE_KERNEL_CONTRACT.to_string(),
        ics20_channel_id: Some("1".to_string()),
        direct_channel_id: Some("2".to_string()),
        supported_modules: vec![],
    };
    CHAIN_TO_CHANNEL
        .save(deps.as_mut().storage, chain, &channel_info)
        .unwrap();

    let username = "username";
    let address = "address";
    let msg = ExecuteMsg::Internal(InternalMsg::RegisterUserCrossChain {
        username: username.to_string(),
        address: address.to_string(),
        chain: chain.to_string(),
    });
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info(MOCK_VFS_CONTRACT, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(res.messages.len(), 1);

    let expected = IbcMsg::SendPacket {
        channel_id: channel_info.direct_channel_id.unwrap(),
        data: to_binary(&IbcExecuteMsg::RegisterUsername {
            username: username.to_string(),
            address: address.to_string(),
        })
        .unwrap(),
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    assert_eq!(res.messages.first().unwrap().msg, CosmosMsg::Ibc(expected));
}

#[test]
fn test_assign_channels() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let env = mock_env();
    let chain = "chain";
    instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        InstantiateMsg {
            owner: None,
            chain_name: "andromeda".to_string(),
        },
    )
    .unwrap();

    let channel_info = ChannelInfo {
        kernel_address: MOCK_FAKE_KERNEL_CONTRACT.to_string(),
        ics20_channel_id: Some("1".to_string()),
        direct_channel_id: Some("2".to_string()),
        supported_modules: vec![],
    };
    let msg = ExecuteMsg::AssignChannels {
        ics20_channel_id: channel_info.ics20_channel_id.clone(),
        direct_channel_id: channel_info.direct_channel_id.clone(),
        chain: chain.to_string(),
        kernel_address: channel_info.kernel_address,
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::AssignChannels {
        ics20_channel_id: None,
        direct_channel_id: Some("3".to_string()),
        chain: chain.to_string(),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };
    execute(deps.as_mut(), env, info, msg).unwrap();

    let channel_info = CHAIN_TO_CHANNEL.load(deps.as_ref().storage, chain).unwrap();
    assert_eq!(
        channel_info.direct_channel_id.clone().unwrap(),
        "3".to_string()
    );
    assert_eq!(
        channel_info.ics20_channel_id.clone().unwrap(),
        "1".to_string()
    );
    assert_eq!(
        channel_info.kernel_address,
        MOCK_KERNEL_CONTRACT.to_string()
    );

    let ics20_channel_chain = CHANNEL_TO_CHAIN
        .load(
            deps.as_ref().storage,
            &channel_info.ics20_channel_id.unwrap(),
        )
        .unwrap();
    assert_eq!(ics20_channel_chain, chain.to_string());
    let direct_channel_chain = CHANNEL_TO_CHAIN
        .load(
            deps.as_ref().storage,
            &channel_info.direct_channel_id.unwrap(),
        )
        .unwrap();
    assert_eq!(direct_channel_chain, chain.to_string());
}

#[test]
fn test_assign_channels_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let env = mock_env();
    let chain = "chain";
    instantiate(
        deps.as_mut(),
        env.clone(),
        info,
        InstantiateMsg {
            owner: None,
            chain_name: "andromeda".to_string(),
        },
    )
    .unwrap();

    let unauth_info = mock_info("attacker", &[]);
    let msg = ExecuteMsg::AssignChannels {
        ics20_channel_id: None,
        direct_channel_id: Some("3".to_string()),
        chain: chain.to_string(),
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };
    let err = execute(deps.as_mut(), env, unauth_info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_send_cross_chain_no_channel() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info(MOCK_VFS_CONTRACT, &[]);
    let env = mock_env();
    instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        InstantiateMsg {
            owner: None,
            chain_name: "andromeda".to_string(),
        },
    )
    .unwrap();
    KERNEL_ADDRESSES
        .save(
            deps.as_mut().storage,
            VFS_KEY,
            &Addr::unchecked(MOCK_VFS_CONTRACT),
        )
        .unwrap();
    let msg = ExecuteMsg::AssignChannels {
        ics20_channel_id: None,
        direct_channel_id: None,
        chain: "chain2".to_string(),
        kernel_address: Addr::unchecked("kernal2").to_string(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    let internal_msg = InternalMsg::RegisterUserCrossChain {
        username: "name".to_string(),
        address: Addr::unchecked("addr").to_string(),
        chain: "chain2".to_string(),
    };
    let msg = ExecuteMsg::Internal(internal_msg);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPacket {
            error: Some("Channel not found for chain chain2".to_string())
        }
    );
}

// Test provided by Quantstamp as part of audit
#[test]
fn test_handle_ibc_direct() {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("user", &[]);
    let env = mock_env();
    let chain = "andromeda";
    instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        InstantiateMsg {
            owner: None,
            chain_name: "andromeda".to_string(),
        },
    )
    .unwrap();
    // Test would fail without this because we need to check the adodb
    let assign_key_msg = ExecuteMsg::UpsertKeyAddress {
        key: ADO_DB_KEY.to_string(),
        value: MOCK_ADODB_CONTRACT.to_string(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), assign_key_msg).unwrap();

    let channel_info = ChannelInfo {
        kernel_address: MOCK_FAKE_KERNEL_CONTRACT.to_string(),
        ics20_channel_id: Some("1".to_string()),
        direct_channel_id: Some("2".to_string()),
        supported_modules: vec![],
    };
    KERNEL_ADDRESSES
        .save(
            deps.as_mut().storage,
            VFS_KEY,
            &Addr::unchecked(MOCK_VFS_CONTRACT),
        )
        .unwrap();
    CHAIN_TO_CHANNEL
        .save(deps.as_mut().storage, chain, &channel_info)
        .unwrap();
    let dummy_msg = ExecuteMsg::UpsertKeyAddress {
        key: "key".to_string(),
        value: "value".to_string(),
    };
    let amp_msg = AMPMsg::new("ibc://andromeda/..", to_binary(&dummy_msg).unwrap(), None);
    let packet = AMPPkt::new("user", "user", vec![amp_msg]);
    let msg = ExecuteMsg::AMPReceive(packet);
    let res = execute(deps.as_mut(), env, info, msg);
    // * message fails even though it is a non-default binary message
    // assert!(res.is_ok());
    // Cross chain components are currently disabled, so the response should be an error
    assert_eq!(
        res.unwrap_err(),
        ContractError::CrossChainComponentsCurrentlyDisabled {}
    )
}
