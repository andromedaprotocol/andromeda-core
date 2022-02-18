use crate::state::{offers, Offer};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, has_coins, Api, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    Order, QuerierWrapper, Reply, Response, StdError, Storage, SubMsg, Uint128,
};

use andromeda_protocol::{
    communication::{
        encode_binary,
        hooks::{AndromedaHook, OnFundsTransferResponse},
        modules::{
            execute_alter_module, execute_deregister_module, execute_register_module, module_hook,
            on_funds_transfer, validate_modules, ADOType, MODULE_ADDR, MODULE_INFO,
        },
        parse_message, AndromedaMsg,
    },
    cw721::{ExecuteMsg, InstantiateMsg, QueryMsg, TokenExtension, TransferAgreement},
    error::ContractError,
    operators::execute_update_operators,
    ownership::{execute_update_owner, CONTRACT_OWNER},
    rates::Funds,
    require,
    response::get_reply_address,
};
use cw721::Expiration;
use cw721_base::{state::TokenInfo, Cw721Contract};
use cw_storage_plus::Bound;

pub type AndrCW721Contract<'a> = Cw721Contract<'a, TokenExtension, Empty>;
const DEFAULT_LIMIT: u32 = 10u32;
const MAX_LIMIT: u32 = 30u32;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;

    let sender = info.sender.as_str();
    let mut resp = Response::default();
    if let Some(modules) = msg.modules.clone() {
        validate_modules(&modules, ADOType::CW721)?;
        for module in modules {
            let response = execute_register_module(
                &deps.querier,
                deps.storage,
                deps.api,
                sender,
                &module,
                ADOType::CW721,
                false,
            )?;
            resp = resp.add_submessages(response.messages);
        }
    }
    let cw721_resp = AndrCW721Contract::default().instantiate(deps, env, info, msg.into())?;
    resp = resp
        .add_attributes(cw721_resp.attributes)
        .add_submessages(cw721_resp.messages);
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    let id = msg.id.to_string();
    require(
        MODULE_INFO.load(deps.storage, &id).is_ok(),
        ContractError::InvalidReplyId {},
    )?;

    let addr = get_reply_address(&msg)?;
    MODULE_ADDR.save(deps.storage, &id, &deps.api.addr_validate(&addr)?)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    module_hook::<Response>(
        deps.storage,
        deps.querier,
        AndromedaHook::OnExecute {
            sender: info.sender.to_string(),
            payload: encode_binary(&msg)?,
        },
    )?;

    // Check if the token is archived before any message that may mutate the token
    match &msg {
        ExecuteMsg::TransferNft { token_id, .. } => {
            is_token_archived(deps.storage, token_id)?;
        }
        ExecuteMsg::SendNft { token_id, .. } => {
            is_token_archived(deps.storage, token_id)?;
        }
        ExecuteMsg::Approve { token_id, .. } => {
            is_token_archived(deps.storage, token_id)?;
        }
        ExecuteMsg::Burn { token_id, .. } => {
            is_token_archived(deps.storage, token_id)?;
        }
        ExecuteMsg::Archive { token_id } => {
            is_token_archived(deps.storage, token_id)?;
        }
        ExecuteMsg::TransferAgreement { token_id, .. } => {
            is_token_archived(deps.storage, token_id)?;
        }
        ExecuteMsg::UpdatePricing { token_id, .. } => {
            is_token_archived(deps.storage, token_id)?;
        }
        _ => {}
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
        ExecuteMsg::UpdatePricing { token_id, price } => {
            execute_update_pricing(deps, env, info, token_id, price)
        }
        ExecuteMsg::Archive { token_id } => execute_archive(deps, env, info, token_id),
        ExecuteMsg::Burn { token_id } => execute_burn(deps, info, token_id),
        ExecuteMsg::RegisterModule { module } => execute_register_module(
            &deps.querier,
            deps.storage,
            deps.api,
            info.sender.as_str(),
            &module,
            ADOType::CW721,
            true,
        ),
        ExecuteMsg::DeregisterModule { module_idx } => {
            execute_deregister_module(deps, info, module_idx)
        }
        ExecuteMsg::AlterModule { module_idx, module } => {
            execute_alter_module(deps, info, module_idx, &module, ADOType::CW721)
        }
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
        _ => Ok(AndrCW721Contract::default().execute(deps, env, info, msg.into())?),
    }
}

fn is_token_archived(storage: &dyn Storage, token_id: &str) -> Result<(), ContractError> {
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(storage, token_id)?;
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;

    Ok(())
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let received: ExecuteMsg = parse_message(data)?;
            match received {
                ExecuteMsg::AndrReceive(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => execute(deps, env, info, received),
            }
        }
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
        AndromedaMsg::Withdraw { .. } => Err(ContractError::UnsupportedOperation {}),
    }
}

fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(deps.storage, &token_id)?;
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;

    let (resp, tax_amount) = if let Some(agreement) = &token.extension.transfer_agreement {
        get_funds_transfer_response_and_taxes(
            deps.storage,
            &deps.querier,
            info.sender.to_string(),
            agreement.amount.clone(),
            token_id.clone(),
            recipient.clone(),
        )?
    } else {
        (Response::new(), Uint128::zero())
    };

    check_can_send(deps.as_ref(), env, info, &token, tax_amount)?;
    transfer_ownership(deps.storage, deps.api, &token_id, &recipient)?;

    Ok(resp
        .add_attribute("action", "transfer")
        .add_attribute("recipient", recipient))
}

fn transfer_ownership(
    storage: &mut dyn Storage,
    api: &dyn Api,
    token_id: &str,
    recipient: &str,
) -> Result<(), ContractError> {
    let contract = AndrCW721Contract::default();
    let mut token = contract.tokens.load(storage, token_id)?;
    token.owner = api.addr_validate(recipient)?;
    token.approvals.clear();
    token.extension.transfer_agreement = None;
    token.extension.pricing = None;
    contract.tokens.save(storage, token_id, &token)?;
    Ok(())
}

fn get_funds_transfer_response_and_taxes(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    sender: String,
    coin: Coin,
    token_id: String,
    recipient: String,
) -> Result<(Response, Uint128), ContractError> {
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(storage, &token_id)?;
    let mut resp = Response::new();
    let (msgs, events, remainder) = on_funds_transfer(
        storage,
        *querier,
        sender,
        Funds::Native(coin.clone()),
        encode_binary(&ExecuteMsg::TransferNft {
            token_id,
            recipient,
        })?,
    )?;
    let remaining_amount = match remainder {
        Funds::Native(coin) => coin, //What do we do in the case that the rates returns remaining amount as non-native funds?
        Funds::Cw20(..) => panic!("Remaining funds returned as incorrect type"),
    };

    let tax_amount = get_tax_amount(&msgs, coin.amount - remaining_amount.amount);

    resp = resp.add_submessages(msgs).add_events(events);
    resp = resp.add_submessage(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: token.owner.to_string(),
        amount: vec![remaining_amount],
    })));
    Ok((resp, tax_amount))
}

fn get_tax_amount(msgs: &[SubMsg], deducted_amount: Uint128) -> Uint128 {
    msgs.iter()
        .map(|msg| {
            if let CosmosMsg::Bank(BankMsg::Send { amount, .. }) = &msg.msg {
                amount[0].amount
            } else {
                Uint128::zero()
            }
        })
        .reduce(|total, amount| total + amount)
        .unwrap_or_else(Uint128::zero)
        - deducted_amount
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
        if agreement.purchaser == info.sender {
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

fn execute_update_pricing(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    pricing: Option<Coin>,
) -> Result<Response, ContractError> {
    let contract = AndrCW721Contract::default();
    let mut token = contract.tokens.load(deps.storage, &token_id)?;
    require(token.owner == info.sender, ContractError::Unauthorized {})?;
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;

    token.extension.pricing = pricing;
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
    require(
        token.owner.eq(&info.sender.to_string()),
        ContractError::Unauthorized {},
    )?;
    require(!token.extension.archived, ContractError::TokenIsArchived {})?;

    contract.tokens.remove(deps.storage, &token_id)?;

    // Decrement token count.
    let count = contract.token_count.load(deps.storage)?;
    contract.token_count.save(deps.storage, &(count - 1))?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "burn"),
        attr("token_id", token_id),
        attr("sender", info.sender.to_string()),
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
            payload,
            amount,
        } => {
            if let Funds::Native(amount) = amount {
                let token_id: String = parse_message(Some(payload))?;
                let (resp, tax_amount) = get_funds_transfer_response_and_taxes(
                    deps.storage,
                    &deps.querier,
                    sender,
                    amount,
                    token_id,
                    // Recipient is unimportant.
                    String::default(),
                )?;
                let res = OnFundsTransferResponse {
                    msgs: resp.messages,
                    events: resp.events,
                    // We may want to alter this based on the sender.
                    payload: encode_binary(&tax_amount)?,
                };
                return Ok(encode_binary(&res)?);
            }
            panic!()
        }
        _ => Err(ContractError::UnsupportedOperation {}),
    }
}
