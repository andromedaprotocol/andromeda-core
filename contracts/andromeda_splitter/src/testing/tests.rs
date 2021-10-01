use crate::{
    msg::{ ExecuteMsg, InstantiateMsg },
    contract::{ execute, instantiate  },
    state:: { SPLITTER, Splitter, STATE, State, AddressPercent }
};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{Addr, Coin, Uint128, Response};
use andromeda_protocol::modules::whitelist::{
    Whitelist, WHITELIST,
};

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg { use_whitelist: true };
    let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_execute_update_lock() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();

    let owner = "creator";
    let info = mock_info(owner.clone(), &[]);

    let lock = true;
    let msg = ExecuteMsg::UpdateLock {
        lock: lock,
    };

    //incorrect owner
    let state = State {
        owner: Addr::unchecked("incorrect_owner".to_string())
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res{
        Ok(_ret) => assert!(false),
        _ => {}
    }

    let state = State {
        owner: Addr::unchecked(owner.to_string())
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let splitter = Splitter {
        recipient: vec![],
        is_lock: false,
        use_whitelist: true,
        sender_whitelist: Whitelist {
                moderators: vec![]
            },
        accepted_tokenlist: vec![],
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(Response::default(), res);

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(splitter.is_lock, lock);
}

#[test]
fn test_execute_update_isusewhitelist() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();

    let owner = "creator";
    let info = mock_info(owner.clone(), &[]);

    let use_whitelist = true;
    let msg = ExecuteMsg::UpdateUseWhitelist {
        use_whitelist,
    };

    //incorrect owner
    let state = State {
        owner: Addr::unchecked("incorrect_owner".to_string())
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res{
        Ok(_ret) => assert!(false),
        _ => {}
    }

    let state = State {
        owner: Addr::unchecked(owner.to_string())
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let splitter = Splitter {
        recipient: vec![],
        is_lock: false,
        use_whitelist: false,
        sender_whitelist: Whitelist {
                moderators: vec![]
            },
        accepted_tokenlist: vec![],
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(Response::default(), res);

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(splitter.use_whitelist, use_whitelist);
}


#[test]
fn test_execute_update_token_list() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();

    let owner = "creator";
    let info = mock_info(owner.clone(), &[]);

    let tokenlist = vec!["11".to_string(),"22".to_string(),"33".to_string()];
    let msg = ExecuteMsg::UpdateTokenList {
        accepted_tokenlist: tokenlist.clone()
    };

    //incorrect owner
    let state = State {
        owner: Addr::unchecked("incorrect_owner".to_string())
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res{
        Ok(_ret) => assert!(false),
        _ => {}
    }

    let state = State {
        owner: Addr::unchecked(owner.to_string())
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let splitter = Splitter {
        recipient: vec![],
        is_lock: false,
        use_whitelist: true,
        sender_whitelist: Whitelist {
            moderators: vec![]
        },
        accepted_tokenlist: vec![],
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(Response::default(), res);

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(splitter.accepted_tokenlist, tokenlist.clone());
}

#[test]
fn test_execute_update_sender_whitelist() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();

    let owner = "creator";
    let info = mock_info(owner.clone(), &[]);

    let sender_whitelist = vec!["11".to_string(),"22".to_string(),"33".to_string()];
    let msg = ExecuteMsg::UpdateSenderWhitelist {
        sender_whitelist: sender_whitelist.clone()
    };

    //incorrect owner
    let state = State {
        owner: Addr::unchecked("incorrect_owner".to_string())
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res{
        Ok(_ret) => assert!(false),
        _ => {}
    }

    let state = State {
        owner: Addr::unchecked(owner.to_string())
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let splitter = Splitter {
        recipient: vec![],
        is_lock: false,
        use_whitelist: true,
        sender_whitelist: Whitelist {
            moderators: vec![]
        },
        accepted_tokenlist: vec![],
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(Response::default(), res);

    //check result
    let whitelisted = WHITELIST
        .load(deps.as_ref().storage, "11".to_string())
        .unwrap();
    assert_eq!(true, whitelisted);

}

#[test]
fn test_execute_update_recipient() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();

    let owner = "creator";
    let info = mock_info(owner.clone(), &[]);

    let recipient = vec![
        AddressPercent{addr:"address1".to_string(),percent:Uint128::from(10 as u128)},
        AddressPercent{addr:"address1".to_string(),percent:Uint128::from(20 as u128)}
    ];
    let msg = ExecuteMsg::UpdateRecipient {
        recipient: recipient.clone()
    };

    //incorrect owner
    let state = State {
        owner: Addr::unchecked("incorrect_owner".to_string())
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res{
        Ok(_ret) => assert!(false),
        _ => {}
    }

    let state = State {
        owner: Addr::unchecked(owner.to_string())
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let splitter = Splitter {
        recipient: vec![],
        is_lock: false,
        use_whitelist: true,
        sender_whitelist: Whitelist {
            moderators: vec![]
        },
        accepted_tokenlist: vec![],
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(Response::default(), res);

    //check result
    let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
    assert_eq!(splitter.recipient, recipient.clone());
}

#[test]
fn test_execute_send() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();

    let owner = "creator";
    let info = mock_info(owner.clone(), &vec![Coin::new(10000, "uluna")]);

    let recipient = vec![
        AddressPercent{addr:"address1".to_string(),percent:Uint128::from(10 as u128)},
        AddressPercent{addr:"address1".to_string(),percent:Uint128::from(20 as u128)}
    ];
    let msg = ExecuteMsg::Send {};

    //incorrect owner
    let state = State {
        owner: Addr::unchecked("incorrect_owner".to_string())
    };
    STATE.save(deps.as_mut().storage, &state).unwrap();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res{
        Ok(_ret) => assert!(false),
        _ => {}
    }

    let state = State {
        owner: Addr::unchecked(owner.to_string())
    };

    STATE.save(deps.as_mut().storage, &state).unwrap();

    let splitter = Splitter {
        recipient: recipient,
        is_lock: false,
        use_whitelist: true,
        sender_whitelist: Whitelist {
            moderators: vec![]
        },
        accepted_tokenlist: vec![],
    };

    SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();
    WHITELIST.save(deps.as_mut().storage, owner.to_string().clone(), &true).unwrap();

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_ne!(Response::default(), res);
}