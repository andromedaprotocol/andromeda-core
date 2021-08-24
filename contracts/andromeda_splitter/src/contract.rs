use cosmwasm_std::{
    entry_point, Deps, DepsMut, Env, MessageInfo, Response, StdResult, StdError, Binary, to_binary,
};
use crate::{
    msg::{ ExecuteMsg, InstantiateMsg, QueryMsg, },
    state::{ State, Splitter, STATE, SPLITTER, AddressPercent, },
};
use andromeda_protocol::{
    require::require,
};
use andromeda_protocol::token::TokenId;
// use std::collections::HashMap; 

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    
    let state = State {
        owner: info.sender,        
    };

    let splitter = Splitter{
        // recipient: HashMap::new(),
        recipient: vec![],
        is_lock: false,
        sender_whitelist: vec![],
        accepted_tokenlist: vec![],
    };
    STATE.save(deps.storage, &state)?;
    SPLITTER.save(deps.storage, &splitter)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateRecipient { recipient } => execute_update_recipient(deps, info, recipient),
        ExecuteMsg::UpdateLock { lock } => execute_update_lock(deps, info, lock),
        ExecuteMsg::UpdateTokenList { accepted_tokenlist } => execute_update_token_list(deps, info, accepted_tokenlist),
        ExecuteMsg::UpdateSenderWhitelist{ sender_whitelist } => execute_update_sender_whitelist( deps, info, sender_whitelist ),
    }
}

fn execute_update_recipient( 
    deps: DepsMut,
    info: MessageInfo,
    recipient: Vec<AddressPercent> ) -> StdResult<Response>
{   
    let state = STATE.load(deps.storage)?;    
    require(state.owner == info.sender,
        StdError::generic_err("Only use by owner")
    )?;

    let mut splitter = SPLITTER.load(deps.storage)?;
    splitter.recipient.clear();
    
    splitter.recipient = recipient.clone();
    // for r in recipient {
    //     splitter.recipient.insert(r.addr, r.percent);
    // }
    
    SPLITTER.save(deps.storage, &splitter)?;
    Ok(Response::default())
}

fn execute_update_lock( 
    deps: DepsMut,
    info: MessageInfo,
    lock: bool ) -> StdResult<Response>
{   
    let state = STATE.load(deps.storage)?;
    require(state.owner == info.sender,
        StdError::generic_err("Only use by owner")
    )?;
    let mut splitter = SPLITTER.load(deps.storage)?;
    splitter.is_lock = lock;
    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default())
}

fn execute_update_token_list( 
    deps: DepsMut,
    info: MessageInfo,
    accepted_tokenlist: Vec<TokenId> ) -> StdResult<Response>
{   
    let state = STATE.load(deps.storage)?;    
    require(state.owner == info.sender,
        StdError::generic_err("Only use by owner")
    )?;
    let mut splitter = SPLITTER.load(deps.storage)?;
    splitter.accepted_tokenlist.clear();
    splitter.accepted_tokenlist = accepted_tokenlist.clone();
    SPLITTER.save(deps.storage, &splitter)?;
    Ok(Response::default())
}

fn execute_update_sender_whitelist( 
    deps: DepsMut,
    info: MessageInfo,
    sender_whitelist: Vec<String> ) -> StdResult<Response>
{   
    let state = STATE.load(deps.storage)?;    
    require(state.owner == info.sender,
        StdError::generic_err("Only use by owner")
    )?;
    let mut splitter = SPLITTER.load(deps.storage)?;
    splitter.sender_whitelist.clear();
    splitter.sender_whitelist = sender_whitelist.clone();
    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Splitter{} => query_splitter(deps),        
    }
}

fn query_splitter(deps: Deps) -> StdResult<Binary> {
    let splitter = SPLITTER.load(deps.storage)?;
    to_binary(&splitter)
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {};
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
            sender_whitelist: vec![],
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
            sender_whitelist: vec![],
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
            sender_whitelist: vec![],
            accepted_tokenlist: vec![],
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default(), res);

        //check result
        let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();        
        assert_eq!(splitter.sender_whitelist, sender_whitelist.clone());        
    }

    #[test]
    fn test_execute_update_recipient() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let owner = "creator";
        let info = mock_info(owner.clone(), &[]);

        
        let recipient = vec![AddressPercent{addr:"11".to_string(),percent:3},AddressPercent{addr:"12".to_string(),percent:5}];
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
            sender_whitelist: vec![],
            accepted_tokenlist: vec![],
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default(), res);

        //check result
        let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();        
        assert_eq!(splitter.recipient, recipient.clone());        
    }

}
