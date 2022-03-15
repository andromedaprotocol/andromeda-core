#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, has_coins, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    Reply, Response, StdError, Storage, SubMsg, Uint128,
};

use ado_base::state::ADOContract;
use andromeda_protocol::{
    cw721::{ExecuteMsg, InstantiateMsg, QueryMsg, TokenExtension, TransferAgreement},
    rates::get_tax_amount,
    response::get_reply_address,
};
use common::{
    ado_base::hooks::{AndromedaHook, OnFundsTransferResponse},
    encode_binary,
    error::ContractError,
    primitive::PRIMITVE_CONTRACT,
    require, Funds,
};
use cw721_base::{state::TokenInfo, Cw721Contract};

pub type AndrCW721Contract<'a> = Cw721Contract<'a, TokenExtension, Empty>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    contract.owner.save(deps.storage, &info.sender)?;
    contract.ado_type.save(deps.storage, &"cw721".to_string())?;
    PRIMITVE_CONTRACT.save(deps.storage, &msg.primitive_contract)?;
    let module_msgs = contract.register_modules(
        info.sender.as_str(),
        &deps.querier,
        deps.storage,
        deps.api,
        msg.modules.clone(),
    )?;

    let cw721_resp = AndrCW721Contract::default().instantiate(deps, env, info, msg.into())?;
    Ok(Response::new()
        .add_attributes(cw721_resp.attributes)
        .add_submessages(module_msgs)
        .add_submessages(cw721_resp.messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    let contract = ADOContract::default();
    let id = msg.id.to_string();
    require(
        contract.module_info.has(deps.storage, &id),
        ContractError::InvalidReplyId {},
    )?;

    let addr = get_reply_address(&msg)?;
    contract
        .module_addr
        .save(deps.storage, &id, &deps.api.addr_validate(&addr)?)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    contract.module_hook::<Response>(
        deps.storage,
        deps.querier,
        AndromedaHook::OnExecute {
            sender: info.sender.to_string(),
            payload: encode_binary(&msg)?,
        },
    )?;

    if let ExecuteMsg::Approve { token_id, .. } = &msg {
        is_token_archived(deps.storage, token_id)?;
    }

    match msg {
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => execute_transfer(deps, env, info, recipient, token_id),
        ExecuteMsg::TransferAgreement {
            token_id,
            agreement,
        } => execute_update_transfer_agreement(deps, env, info, token_id, agreement),
        ExecuteMsg::Archive { token_id } => execute_archive(deps, env, info, token_id),
        ExecuteMsg::Burn { token_id } => execute_burn(deps, info, token_id),
        ExecuteMsg::AndrReceive(msg) => contract.execute(deps, env, info, msg, execute),
        _ => Ok(AndrCW721Contract::default().execute(deps, env, info, msg.into())?),
    }
}

fn is_token_archived(storage: &dyn Storage, token_id: &str) -> Result<(), ContractError> {
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(storage, token_id)?;
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;

    Ok(())
}

fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let base_contract = ADOContract::default();
    let responses = base_contract.module_hook::<Response>(
        deps.storage,
        deps.querier,
        AndromedaHook::OnTransfer {
            token_id: token_id.clone(),
            sender: info.sender.to_string(),
            recipient: recipient.clone(),
        },
    )?;
    // Reduce all responses into one.
    let mut resp = responses
        .into_iter()
        .reduce(|resp, r| {
            resp.add_submessages(r.messages)
                .add_events(r.events)
                .add_attributes(r.attributes)
        })
        .unwrap_or_else(Response::new);

    let contract = AndrCW721Contract::default();
    let mut token = contract.tokens.load(deps.storage, &token_id)?;
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;

    let tax_amount = if let Some(agreement) = &token.extension.transfer_agreement {
        let (mut msgs, events, remainder) = base_contract.on_funds_transfer(
            deps.storage,
            deps.querier,
            info.sender.to_string(),
            Funds::Native(agreement.amount.clone()),
            encode_binary(&ExecuteMsg::TransferNft {
                token_id: token_id.clone(),
                recipient: recipient.clone(),
            })?,
        )?;
        let remaining_amount = remainder.try_get_coin()?;
        let tax_amount = get_tax_amount(&msgs, agreement.amount.amount - remaining_amount.amount);
        msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: token.owner.to_string(),
            amount: vec![remaining_amount],
        })));
        resp = resp.add_submessages(msgs).add_events(events);
        tax_amount
    } else {
        Uint128::zero()
    };

    check_can_send(deps.as_ref(), env, info, &token, tax_amount)?;
    token.owner = deps.api.addr_validate(&recipient)?;
    token.approvals.clear();
    token.extension.transfer_agreement = None;
    token.extension.pricing = None;
    contract.tokens.save(deps.storage, &token_id, &token)?;
    Ok(resp
        .add_attribute("action", "transfer")
        .add_attribute("recipient", recipient))
}

fn check_can_send(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    token: &TokenInfo<TokenExtension>,
    tax_amount: Uint128,
) -> Result<(), ContractError> {
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;
    // owner can send
    if token.owner == info.sender {
        return Ok(());
    }

    // token purchaser can send if correct funds are sent
    if let Some(agreement) = &token.extension.transfer_agreement {
        require(
            has_coins(
                &info.funds,
                &Coin {
                    denom: agreement.amount.denom.to_owned(),
                    // Ensure that the taxes came from the sender.
                    amount: agreement.amount.amount + tax_amount,
                },
            ),
            ContractError::InsufficientFunds {},
        )?;
        if agreement.purchaser == info.sender || agreement.purchaser == "*" {
            return Ok(());
        }
    }

    // any non-expired token approval can send
    if token
        .approvals
        .iter()
        .any(|apr| apr.spender == info.sender && !apr.is_expired(&env.block))
    {
        return Ok(());
    }

    // operator can send
    let op = AndrCW721Contract::default()
        .operators
        .may_load(deps.storage, (&token.owner, &info.sender))?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(())
            }
        }
        None => Err(ContractError::Unauthorized {}),
    }
}

fn execute_update_transfer_agreement(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    agreement: Option<TransferAgreement>,
) -> Result<Response, ContractError> {
    let contract = AndrCW721Contract::default();
    let mut token = contract.tokens.load(deps.storage, &token_id)?;
    require(token.owner == info.sender, ContractError::Unauthorized {})?;
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;
    if let Some(xfer_agreement) = agreement.clone() {
        deps.api.addr_validate(&xfer_agreement.purchaser)?;
    }

    token.extension.transfer_agreement = agreement;
    contract
        .tokens
        .save(deps.storage, token_id.as_str(), &token)?;

    Ok(Response::default())
}

fn execute_archive(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let contract = AndrCW721Contract::default();
    let mut token = contract.tokens.load(deps.storage, &token_id)?;
    require(token.owner == info.sender, ContractError::Unauthorized {})?;

    token.extension.archived = true;
    contract
        .tokens
        .save(deps.storage, token_id.as_str(), &token)?;

    Ok(Response::default())
}

fn execute_burn(
    deps: DepsMut,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(deps.storage, &token_id)?;
    require(token.owner == info.sender, ContractError::Unauthorized {})?;
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;

    contract.tokens.remove(deps.storage, &token_id)?;

    // Decrement token count.
    let count = contract.token_count.load(deps.storage)?;
    contract.token_count.save(deps.storage, &(count - 1))?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "burn"),
        attr("token_id", token_id),
        attr("sender", info.sender),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrHook(msg) => handle_andr_hook(deps, msg),
        _ => Ok(AndrCW721Contract::default().query(deps, env, msg.into())?),
    }
}

fn handle_andr_hook(deps: Deps, msg: AndromedaHook) -> Result<Binary, ContractError> {
    match msg {
        AndromedaHook::OnFundsTransfer {
            sender,
            payload: _,
            amount,
        } => {
            let (msgs, events, remainder) = ADOContract::default().on_funds_transfer(
                deps.storage,
                deps.querier,
                sender,
                amount,
                encode_binary(&String::default())?,
            )?;
            let res = OnFundsTransferResponse {
                msgs,
                events,
                leftover_funds: remainder,
            };
            Ok(encode_binary(&res)?)
        }
        _ => Err(ContractError::UnsupportedOperation {}),
    }
}
