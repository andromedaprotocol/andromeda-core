use cosmwasm_std::{
    entry_point, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    StdError, Binary, to_binary, Coin, SubMsg, CosmosMsg, BankMsg,
};
use crate::{
    msg::{ ExecuteMsg, InstantiateMsg, QueryMsg, IsWhitelistedResponse, },
    state::{ State, Splitter, STATE, SPLITTER, AddressPercent, },
};
use andromeda_protocol::{
    require::require,
};
use andromeda_protocol::token::TokenId;
use andromeda_protocol::modules::whitelist::{
    Whitelist
};
// use std::collections::HashMap;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        owner: info.sender,
    };

    let splitter = Splitter{
        recipient: vec![],
        is_lock: false,
        use_whitelist: msg.use_whitelist,
        sender_whitelist: Whitelist {
            moderators: vec![]
        },
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
        ExecuteMsg::UpdateUseWhitelist{ use_whitelist } => execute_update_isusewhitelist(deps, info, use_whitelist ),
        ExecuteMsg::UpdateRecipient { recipient } => execute_update_recipient( deps, info, recipient ),
        ExecuteMsg::UpdateLock { lock } => execute_update_lock( deps, info, lock ),
        ExecuteMsg::UpdateTokenList { accepted_tokenlist } => execute_update_token_list( deps, info, accepted_tokenlist ),
        ExecuteMsg::UpdateSenderWhitelist{ sender_whitelist } => execute_update_sender_whitelist( deps, info, sender_whitelist ),
        ExecuteMsg::Send{} => execute_send( deps, info ),
    }
}

fn execute_update_isusewhitelist(
    deps: DepsMut,
    info: MessageInfo,
    use_whitelist: bool
) -> StdResult<Response>{
    let state = STATE.load(deps.storage)?;
    require(state.owner == info.sender,
        StdError::generic_err("Only use by owner")
    )?;

    let mut splitter = SPLITTER.load(deps.storage)?;

    if splitter.is_lock == true {
        StdError::generic_err("Not allow to change recipient");
    }

    splitter.use_whitelist = use_whitelist;

    SPLITTER.save(deps.storage, &splitter)?;
    Ok(Response::default())
}
fn execute_send( deps: DepsMut, info: MessageInfo ) -> StdResult<Response>{
    let sent_funds:Vec<Coin> = info.funds.clone();

    require( sent_funds.len() > 0,
        StdError::generic_err("No coin sent")
    )?;

    let splitter = SPLITTER.load(deps.storage)?;

    if splitter.use_whitelist == true {
    require ( splitter.sender_whitelist.is_whitelisted(deps.storage, &info.sender.to_string())?,
        StdError::generic_err("Not allow to send funds")
    )?;
    }

    require( splitter.recipient.len() > 0,
        StdError::generic_err("No recipient received")
    )?;

    let mut submsg:Vec<SubMsg> = Vec::new();

    for recipient_addr in splitter.recipient{
        let recipient_percent = recipient_addr.percent;
        let mut vec_coin: Vec<Coin> = Vec::new();
        for coin in &sent_funds{
            let mut recip_coin: Coin = coin.clone();
            recip_coin.amount = coin.amount.multiply_ratio(recipient_percent, 100 as u128);
            vec_coin.push(recip_coin);
        }
        submsg.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient_addr.addr,
                amount: vec_coin,
        })));
    }

    Ok( Response::new().add_submessages(submsg) )
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

    if splitter.is_lock == true {
        StdError::generic_err("Not allow to change recipient");
    }

    splitter.recipient.clear();

    splitter.recipient = recipient.clone();
    SPLITTER.save(deps.storage, &splitter)?;
    Ok(Response::default())
}

fn execute_update_lock(
    deps: DepsMut,
    info: MessageInfo,
    lock: bool
) -> StdResult<Response>
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
    accepted_tokenlist: Vec<TokenId>
) -> StdResult<Response>
{
    let state = STATE.load(deps.storage)?;
    require(state.owner == info.sender,
        StdError::generic_err("Only use by owner")
    )?;
    let mut splitter = SPLITTER.load(deps.storage)?;

    if splitter.is_lock == true {
        StdError::generic_err("Not allow to change token list");
    }

    splitter.accepted_tokenlist.clear();
    splitter.accepted_tokenlist = accepted_tokenlist.clone();
    SPLITTER.save(deps.storage, &splitter)?;
    Ok(Response::default())
}

fn execute_update_sender_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    sender_whitelist: Vec<String>
) -> StdResult<Response>
{
    let state = STATE.load(deps.storage)?;
    require(state.owner == info.sender,
        StdError::generic_err("Only use by owner")
    )?;
    let splitter = SPLITTER.load(deps.storage)?;

    if splitter.is_lock == true {
        StdError::generic_err("Not allow to change whitelist");
    }

    // splitter.sender_whitelist.clear();
    for whitelist_item in sender_whitelist{
        splitter
            .sender_whitelist
            .whitelist_addr(deps.storage, &whitelist_item)
            .unwrap();
    }

    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Splitter{} => query_splitter(deps),
        QueryMsg::IsWhitelisted { address } => query_splitter_whitelist(deps, &address),
    }
}
fn query_splitter_whitelist(deps: Deps, address: &String) -> StdResult<Binary>{
    let splitter = SPLITTER.load(deps.storage)?;

    to_binary(&IsWhitelistedResponse {
        whitelisted: splitter.sender_whitelist.is_whitelisted(deps.storage, address)?,
    })

}

fn query_splitter(deps: Deps) -> StdResult<Binary> {
    let splitter = SPLITTER.load(deps.storage)?;
    to_binary(&splitter)
}