#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, has_coins, to_binary, Addr, Api, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty,
    Env, MessageInfo, QuerierWrapper, Response, Storage, SubMsg, Uint128,
};

use crate::state::{is_archived, ANDR_MINTER, ARCHIVED, TRANSFER_AGREEMENTS};
use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::cw721::{
    ExecuteMsg, InstantiateMsg, QueryMsg, TokenExtension, TransferAgreement,
};
use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        AndromedaMsg, InstantiateMsg as BaseInstantiateMsg,
    },
    encode_binary,
    error::ContractError,
    rates::get_tax_amount,
    require, Funds,
};
use cw721::ContractInfoResponse;
use cw721_base::{state::TokenInfo, Cw721Contract};

pub type AndrCW721Contract<'a> = Cw721Contract<'a, TokenExtension, Empty>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract_info = ContractInfoResponse {
        name: msg.name,
        symbol: msg.symbol,
    };
    // Do this directly instead of with cw721_contract.instantiate because we want to have minter
    // be an AndrAddress, which cannot be validated right away.
    AndrCW721Contract::default()
        .contract_info
        .save(deps.storage, &contract_info)?;

    ANDR_MINTER.save(deps.storage, &msg.minter)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "cw721".to_string(),
            operators: None,
            modules: msg.modules.clone(),
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    // Do this before the hooks get fired off to ensure that there are no errors from the app
    // address not being fully setup yet.
    if let ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract { address }) = msg {
        let andr_minter = ANDR_MINTER.load(deps.storage)?;
        return contract.execute_update_app_contract(deps, info, address, Some(vec![andr_minter]));
    };

    contract.module_hook::<Response>(
        deps.storage,
        deps.api,
        deps.querier,
        AndromedaHook::OnExecute {
            sender: info.sender.to_string(),
            payload: encode_binary(&msg)?,
        },
    )?;

    if let ExecuteMsg::Approve { token_id, .. } = &msg {
        require(
            !is_archived(deps.storage, token_id)?,
            ContractError::TokenIsArchived {},
        )?;
    }

    match msg {
        ExecuteMsg::Mint(_) => execute_mint(deps, env, info, msg),
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

fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let cw721_contract = AndrCW721Contract::default();
    let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
    let andr_minter = ANDR_MINTER.load(deps.storage)?;
    if cw721_contract.minter.may_load(deps.storage)?.is_none() {
        let addr = deps.api.addr_validate(&andr_minter.get_address(
            deps.api,
            &deps.querier,
            app_contract,
        )?)?;
        save_minter(&cw721_contract, deps.storage, &addr)?;
    }
    Ok(cw721_contract.execute(deps, env, info, msg.into())?)
}

fn save_minter(
    cw721_contract: &AndrCW721Contract,
    storage: &mut dyn Storage,
    minter: &Addr,
) -> Result<(), ContractError> {
    Ok(cw721_contract.minter.save(storage, minter)?)
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
        deps.api,
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
    require(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {},
    )?;

    let tax_amount = if let Some(agreement) =
        &TRANSFER_AGREEMENTS.may_load(deps.storage, &token_id)?
    {
        let app_contract = base_contract.get_app_contract(deps.storage)?;
        let agreement_amount =
            get_transfer_agreement_amount(deps.api, &deps.querier, app_contract, agreement)?;
        let (mut msgs, events, remainder) = base_contract.on_funds_transfer(
            deps.storage,
            deps.api,
            &deps.querier,
            info.sender.to_string(),
            Funds::Native(agreement_amount.clone()),
            encode_binary(&ExecuteMsg::TransferNft {
                token_id: token_id.clone(),
                recipient: recipient.clone(),
            })?,
        )?;
        let remaining_amount = remainder.try_get_coin()?;
        let tax_amount = get_tax_amount(&msgs, agreement_amount.amount, remaining_amount.amount);
        msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: token.owner.to_string(),
            amount: vec![remaining_amount],
        })));
        resp = resp.add_submessages(msgs).add_events(events);
        tax_amount
    } else {
        Uint128::zero()
    };

    require(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {},
    )?;
    check_can_send(deps.as_ref(), env, info, &token_id, &token, tax_amount)?;
    token.owner = deps.api.addr_validate(&recipient)?;
    token.approvals.clear();
    contract.tokens.save(deps.storage, &token_id, &token)?;
    Ok(resp
        .add_attribute("action", "transfer")
        .add_attribute("recipient", recipient))
}

fn get_transfer_agreement_amount(
    api: &dyn Api,
    querier: &QuerierWrapper,
    app_contract: Option<Addr>,
    agreement: &TransferAgreement,
) -> Result<Coin, ContractError> {
    let agreement_amount = agreement
        .amount
        .clone()
        .try_into_coin(api, querier, app_contract)?;
    match agreement_amount {
        Some(amount) => Ok(amount),
        None => Err(ContractError::PrimitiveDoesNotExist {
            msg: "TransferAgreement price is None".to_string(),
        }),
    }
}

fn check_can_send(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    token_id: &str,
    token: &TokenInfo<TokenExtension>,
    tax_amount: Uint128,
) -> Result<(), ContractError> {
    // owner can send
    if token.owner == info.sender {
        return Ok(());
    }

    // token purchaser can send if correct funds are sent
    if let Some(agreement) = &TRANSFER_AGREEMENTS.may_load(deps.storage, token_id)? {
        let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
        let agreement_amount =
            get_transfer_agreement_amount(deps.api, &deps.querier, app_contract, agreement)?;
        require(
            has_coins(
                &info.funds,
                &Coin {
                    denom: agreement_amount.denom.to_owned(),
                    // Ensure that the taxes came from the sender.
                    amount: agreement_amount.amount + tax_amount,
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
    let token = contract.tokens.load(deps.storage, &token_id)?;
    require(token.owner == info.sender, ContractError::Unauthorized {})?;
    require(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {},
    )?;
    if let Some(xfer_agreement) = &agreement {
        TRANSFER_AGREEMENTS.save(deps.storage, &token_id, xfer_agreement)?;
        if xfer_agreement.purchaser != "*" {
            deps.api.addr_validate(&xfer_agreement.purchaser)?;
        }
    } else {
        TRANSFER_AGREEMENTS.remove(deps.storage, &token_id);
    }

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
    require(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {},
    )?;
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(deps.storage, &token_id)?;
    require(token.owner == info.sender, ContractError::Unauthorized {})?;

    ARCHIVED.save(deps.storage, &token_id, &true)?;

    contract.tokens.save(deps.storage, &token_id, &token)?;

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
    require(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {},
    )?;

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
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::IsArchived { token_id } => Ok(to_binary(&is_archived(deps.storage, &token_id)?)?),
        QueryMsg::TransferAgreement { token_id } => {
            Ok(to_binary(&query_transfer_agreement(deps, token_id)?)?)
        }
        _ => Ok(AndrCW721Contract::default().query(deps, env, msg.into())?),
    }
}

pub fn query_transfer_agreement(
    deps: Deps,
    token_id: String,
) -> Result<Option<TransferAgreement>, ContractError> {
    Ok(TRANSFER_AGREEMENTS.may_load(deps.storage, &token_id)?)
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
                deps.api,
                &deps.querier,
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
