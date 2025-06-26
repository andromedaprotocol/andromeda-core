use andromeda_non_fungible_tokens::cw721::{BatchSendMsg, MintMsg, TransferAgreement};
#[cfg(not(feature = "library"))]
use andromeda_non_fungible_tokens::cw721::{ExecuteMsg, InstantiateMsg, QueryMsg};

use cosmwasm_std::{
    attr, ensure, entry_point, has_coins, to_json_binary, Addr, Api, BankMsg, Binary, Coin,
    CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Response, StdError, SubMsg,
    Uint128,
};

use crate::state::{is_archived, ANDR_MINTER, ARCHIVED, TRANSFER_AGREEMENTS};

use cw721::{
    execute::{
        approve, approve_all, burn_nft, instantiate as cw721_instantiate, revoke, revoke_all,
        send_nft, transfer_nft,
    },
    msg::{Cw721InstantiateMsg, OwnerOfResponse},
    query::{
        query_all_nft_info, query_all_tokens, query_approval, query_approvals,
        query_collection_info, query_minter_ownership, query_nft_info, query_num_tokens,
        query_operators, query_owner_of, query_tokens,
    },
    state::{Cw721Config, NftInfo},
    Approval,
};

use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::{permissioning::is_context_permissioned, ADOContract},
    amp::AndrAddr,
    andr_execute_fn,
    common::{context::ExecuteContext, rates::get_tax_amount, Funds},
    error::ContractError,
};

const CONTRACT_NAME: &str = "crates.io:andromeda-cw721";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MINT_ACTION: &str = "Mint";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let minter = msg.minter.get_raw_address(&deps.as_ref())?.into_string();
    let cw721_instantiate_msg = Cw721InstantiateMsg {
        creator: msg.owner.clone(),
        withdraw_address: None,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        minter: Some(minter),
    };
    ANDR_MINTER.save(deps.storage, &msg.minter)?;

    let contract = ADOContract::default();

    contract.permission_action(deps.storage, MINT_ACTION)?;

    let resp = contract.instantiate(
        deps.storage,
        env.clone(),
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;

    let res = cw721_instantiate(deps.branch(), &env, &info, cw721_instantiate_msg)?;

    Ok(res
        .add_attribute("minter", msg.minter.to_string())
        .add_submessages(resp.messages)
        .add_events(resp.events)
        .set_data(resp.data.unwrap_or_default())
        .add_attributes(resp.attributes))
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    if let ExecuteMsg::Approve { token_id, .. } = &msg {
        ensure!(
            !is_archived(ctx.deps.storage, token_id)?.is_archived,
            ContractError::TokenIsArchived {}
        );
    }

    let res = match msg {
        ExecuteMsg::Mint {
            token_id,
            token_uri,
            owner,
        } => execute_mint(ctx, token_id, token_uri, owner),
        ExecuteMsg::BatchMint { tokens } => execute_batch_mint(ctx, tokens),
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => execute_transfer(ctx, recipient, token_id),
        ExecuteMsg::TransferAgreement {
            token_id,
            agreement,
        } => execute_update_transfer_agreement(ctx, token_id, agreement),
        ExecuteMsg::Archive { token_id } => execute_archive(ctx, token_id),
        ExecuteMsg::Burn { token_id } => execute_burn(ctx, token_id),
        ExecuteMsg::SendNft {
            contract,
            token_id,
            msg,
        } => execute_send_nft(ctx, token_id, contract, msg),
        ExecuteMsg::BatchSend { batch } => execute_batch_send_nft(ctx, batch),
        ExecuteMsg::Approve {
            spender,
            token_id,
            expires,
        } => {
            let res = approve(ctx.deps, &ctx.env, &ctx.info, spender, token_id, expires)?;
            Ok(res)
        }
        ExecuteMsg::Revoke { spender, token_id } => {
            let res = revoke(ctx.deps, &ctx.env, &ctx.info, spender, token_id)?;
            Ok(res)
        }
        ExecuteMsg::ApproveAll { operator, expires } => {
            let res = approve_all(ctx.deps, &ctx.env, &ctx.info, operator, expires)?;
            Ok(res)
        }
        ExecuteMsg::RevokeAll { operator } => {
            let res = revoke_all(ctx.deps, &ctx.env, &ctx.info, operator)?;
            Ok(res)
        }
        _ => ADOContract::default().execute(ctx, msg),
    }?;
    Ok(res)
}

macro_rules! ensure_can_mint {
    ($ctx:expr) => {
        let minter = ANDR_MINTER
            .load($ctx.deps.storage)?
            .get_raw_address(&$ctx.deps.as_ref())?;

        let is_minter = $ctx.info.sender == minter;
        // We check if the sender is the minter before checking if they have the mint permission
        // to prevent consuming unnecessary limited permission usage.
        let has_mint_permission = is_minter
            || is_context_permissioned(
                &mut $ctx.deps,
                &$ctx.info,
                &$ctx.env,
                &$ctx.amp_ctx,
                MINT_ACTION,
            )?;

        ensure!(has_mint_permission, ContractError::Unauthorized {});
    };
}

fn execute_mint(
    mut ctx: ExecuteContext,
    token_id: String,
    token_uri: Option<String>,
    owner: AndrAddr,
) -> Result<Response, ContractError> {
    ensure_can_mint!(ctx);

    let owner: Addr = owner.get_raw_address(&ctx.deps.as_ref())?;
    let token_msg: NftInfo = NftInfo {
        owner: owner.clone(),
        approvals: vec![],
        token_uri: token_uri.clone(),
    };

    let config = Cw721Config::default();

    config
        .nft_info
        .update(ctx.deps.storage, &token_id, |old| match old {
            Some(_) => Err(ContractError::Claimed {}),
            None => Ok(token_msg),
        })?;

    config.increment_tokens(ctx.deps.storage)?;

    let res = Response::new()
        .add_attribute("action", "mint")
        .add_attribute("minter", ctx.info.sender.to_string())
        .add_attribute("owner", owner)
        .add_attribute("token_id", token_id);
    Ok(res)
}

fn execute_batch_mint(
    mut ctx: ExecuteContext,
    tokens_to_mint: Vec<MintMsg>,
) -> Result<Response, ContractError> {
    let mut resp = Response::default();
    ensure_can_mint!(ctx);
    ensure!(
        !tokens_to_mint.is_empty(),
        ContractError::new("No tokens to mint")
    );
    for msg in tokens_to_mint {
        let mut ctx = ExecuteContext::new(ctx.deps.branch(), ctx.info.clone(), ctx.env.clone());
        ctx.amp_ctx = ctx.amp_ctx.clone();
        let mint_resp = execute_mint(ctx, msg.token_id, msg.token_uri, msg.owner.into())?;
        resp = resp
            .add_attributes(mint_resp.attributes)
            .add_submessages(mint_resp.messages);
    }

    Ok(resp)
}

fn execute_transfer(
    ctx: ExecuteContext,
    recipient: AndrAddr,
    token_id: String,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        mut deps,
        info,
        env,
        ..
    } = ctx;
    let base_contract = ADOContract::default();
    let mut resp = Response::new();

    let OwnerOfResponse { owner, .. } =
        query_owner_of(deps.as_ref(), &env, token_id.clone(), false)?;
    ensure!(
        !is_archived(deps.storage, &token_id)?.is_archived,
        ContractError::TokenIsArchived {}
    );

    let tax_amount = if let Some(agreement) =
        &TRANSFER_AGREEMENTS.may_load(deps.storage, &token_id)?
    {
        let agreement_amount = get_transfer_agreement_amount(deps.api, &deps.querier, agreement)?;
        let transfer_response = base_contract.query_deducted_funds(
            deps.as_ref(),
            "Transfer",
            Funds::Native(agreement_amount.clone()),
        )?;

        match transfer_response {
            Some(mut transfer_response) => {
                let remaining_amount = transfer_response.leftover_funds.try_get_coin()?;
                let tax_amount = get_tax_amount(
                    &transfer_response.msgs,
                    agreement_amount.amount,
                    remaining_amount.amount,
                );
                transfer_response
                    .msgs
                    .push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                        to_address: owner.clone(),
                        amount: vec![remaining_amount],
                    })));
                resp = resp.add_submessages(transfer_response.msgs);
                tax_amount
            }
            None => {
                let remaining_amount = Funds::Native(agreement_amount).try_get_coin()?;
                let tax_amount = Uint128::zero();
                let msg: SubMsg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: owner.clone(),
                    amount: vec![remaining_amount],
                }));
                resp = resp.add_submessage(msg);
                tax_amount
            }
        }
    } else {
        Uint128::zero()
    };

    let approvals = query_approvals(deps.as_ref(), &env, token_id.clone(), true)?;
    let operators = query_operators(deps.as_ref(), &env, owner.clone(), true, None, None)?;
    check_can_send(
        deps.as_ref(),
        env.clone(),
        info.clone(),
        &token_id,
        tax_amount,
        owner.clone(),
        approvals.approvals,
        operators.operators,
    )?;

    // If we reach here we can assume the sender is authorised to transfer the NFT
    // We mock message info to have the owner of the NFT be the sender to authorise send.
    let mut transfer_info = info.clone();
    transfer_info.sender = Addr::unchecked(owner);
    let recipient_address = recipient.get_raw_address(&deps.as_ref())?.into_string();
    transfer_nft(
        deps.branch(),
        &env,
        &transfer_info,
        recipient_address.as_str(),
        &token_id,
    )?;
    TRANSFER_AGREEMENTS.remove(deps.storage, &token_id);

    // Extract elements from the response and include them in the final response
    let response = resp.clone();
    for attr in response.attributes.clone() {
        resp = resp.add_attribute(attr.key, attr.value);
    }
    for event in response.events.clone() {
        resp = resp.add_event(event);
    }
    for submsg in response.messages.clone() {
        resp = resp.add_submessage(submsg);
    }
    Ok(response
        .add_attribute("action", "transfer")
        .add_attribute("recipient", recipient_address))
}

fn get_transfer_agreement_amount(
    _api: &dyn Api,
    _querier: &QuerierWrapper,
    agreement: &TransferAgreement,
) -> Result<Coin, ContractError> {
    let agreement_amount = agreement.amount.clone();
    Ok(agreement_amount)
}

#[allow(clippy::too_many_arguments)]
fn check_can_send(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    token_id: &str,
    tax_amount: Uint128,
    owner: String,
    approvals: Vec<Approval>,
    operators: Vec<Approval>,
) -> Result<(), ContractError> {
    // owner can send
    if owner == info.sender.as_str() {
        return Ok(());
    }

    // token purchaser can send if correct funds are sent
    if let Some(agreement) = &TRANSFER_AGREEMENTS.may_load(deps.storage, token_id)? {
        let agreement_amount = get_transfer_agreement_amount(deps.api, &deps.querier, agreement)?;
        ensure!(
            has_coins(
                &info.funds,
                &Coin {
                    denom: agreement_amount.denom.to_owned(),
                    // Ensure that the taxes came from the sender.
                    amount: agreement_amount.amount + tax_amount,
                },
            ),
            ContractError::InsufficientFunds {}
        );
        if agreement.purchaser == info.sender.as_str() || agreement.purchaser == "*" {
            return Ok(());
        }
    }

    // any non-expired token approval can send
    if approvals
        .iter()
        .any(|apr| apr.spender == info.sender && !apr.is_expired(&env.block))
    {
        return Ok(());
    }

    // operator can send
    let op = operators
        .iter()
        .find(|op| op.spender == info.sender && !op.is_expired(&env.block));
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
    ctx: ExecuteContext,
    token_id: String,
    agreement: Option<TransferAgreement>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, ref info, .. } = ctx;
    let token_owner = query_owner_of(deps.as_ref(), &ctx.env, token_id.clone(), false)?;
    ensure!(
        token_owner.owner == info.sender.as_ref(),
        ContractError::Unauthorized {}
    );
    ensure!(
        !is_archived(deps.storage, &token_id)?.is_archived,
        ContractError::TokenIsArchived {}
    );
    if let Some(xfer_agreement) = &agreement {
        TRANSFER_AGREEMENTS.save(deps.storage, &token_id, xfer_agreement)?;
        if xfer_agreement.purchaser != "*" {
            deps.api.addr_validate(&xfer_agreement.purchaser)?;
            approve(
                deps,
                &ctx.env,
                &ctx.info,
                xfer_agreement.purchaser.clone(),
                token_id.clone(),
                None,
            )?;
        }
    } else {
        TRANSFER_AGREEMENTS.remove(deps.storage, &token_id);
        revoke(
            deps,
            &ctx.env,
            &ctx.info,
            info.sender.to_string(),
            token_id.clone(),
        )?;
    }

    let mut attributes = vec![
        attr("action", "update_transfer_agreement"),
        attr("token_id", token_id),
    ];

    if let Some(xfer_agreement) = &agreement {
        attributes.push(attr("purchaser", &xfer_agreement.purchaser));
        attributes.push(attr("amount", xfer_agreement.amount.to_string()));
    } else {
        attributes.push(attr("agreement", "removed"));
    }

    Ok(Response::default().add_attributes(attributes))
}

fn execute_archive(ctx: ExecuteContext, token_id: String) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    ensure!(
        !is_archived(deps.storage, &token_id)?.is_archived,
        ContractError::TokenIsArchived {}
    );
    let token_owner = query_owner_of(deps.as_ref(), &ctx.env, token_id.clone(), false)?;
    ensure!(
        token_owner.owner == info.sender.as_ref(),
        ContractError::Unauthorized {}
    );

    ARCHIVED.save(deps.storage, &token_id, &true)?;

    Ok(Response::default())
}

fn execute_burn(ctx: ExecuteContext, token_id: String) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;

    // let token = contract.tokens.load(deps.storage, &token_id)?;
    let token_owner = query_owner_of(deps.as_ref(), &env, token_id.clone(), false)?;
    ensure!(
        token_owner.owner == info.sender.as_ref(),
        ContractError::Unauthorized {}
    );
    ensure!(
        !is_archived(deps.storage, &token_id)?.is_archived,
        ContractError::TokenIsArchived {}
    );
    let res = burn_nft(deps, &env, &info, token_id)?;
    Ok(res)
}

fn execute_send_nft(
    ctx: ExecuteContext,
    token_id: String,
    contract_addr: AndrAddr,
    msg: Binary,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    TRANSFER_AGREEMENTS.remove(deps.storage, &token_id);
    let contract_addr = contract_addr.get_raw_address(&deps.as_ref())?.into_string();

    let res = send_nft(deps, &env, &info, contract_addr, token_id, msg)?;
    Ok(res)
}

fn execute_batch_send_nft(
    mut ctx: ExecuteContext,
    batch: Vec<BatchSendMsg>,
) -> Result<Response, ContractError> {
    ensure!(!batch.is_empty(), ContractError::EmptyBatch {});

    let mut resp = Response::default();
    for item in batch {
        let mut ctx = ExecuteContext::new(ctx.deps.branch(), ctx.info.clone(), ctx.env.clone());
        ctx.amp_ctx = ctx.amp_ctx.clone();
        let contract_addr = item
            .contract_addr
            .get_raw_address(&ctx.deps.as_ref())?
            .into_string();
        let send_resp = send_nft(
            ctx.deps,
            &ctx.env,
            &ctx.info,
            contract_addr,
            item.token_id,
            item.msg,
        )?;
        resp = resp
            .add_attributes(send_resp.attributes)
            .add_submessages(send_resp.messages);
    }

    Ok(resp.add_attribute("action", "batch_send_nft"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::IsArchived { token_id } => {
            let res = is_archived(deps.storage, &token_id)?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::TransferAgreement { token_id } => {
            let res = query_transfer_agreement(deps, token_id)?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::Minter {} => {
            let res = query_minter(deps)?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::OwnerOf {
            token_id,
            include_expired,
        } => {
            let res = query_owner_of(deps, &env, token_id, include_expired.unwrap_or(false))?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::AllOperators {
            owner,
            include_expired,
            start_after,
            limit,
        } => {
            let res = query_operators(
                deps,
                &env,
                owner,
                include_expired.unwrap_or(false),
                start_after,
                limit,
            )?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::NumTokens {} => {
            let res = query_num_tokens(deps.storage)?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::NftInfo { token_id } => {
            let res = query_nft_info(deps.storage, token_id)?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::AllNftInfo {
            token_id,
            include_expired,
        } => {
            let res = query_all_nft_info(deps, &env, token_id, include_expired.unwrap_or(false))?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => {
            let res = query_tokens(deps, &env, owner, start_after, limit)?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::AllTokens { start_after, limit } => {
            let res = query_all_tokens(deps, &env, start_after, limit)?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::ContractInfo {} => {
            let res = query_collection_info(deps.storage)?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::Approval {
            token_id,
            spender,
            include_expired,
        } => {
            let spender = deps.api.addr_validate(&spender)?;
            let res = query_approval(
                deps,
                &env,
                token_id,
                spender,
                include_expired.unwrap_or(false),
            )?;
            Ok(to_json_binary(&res)?)
        }
        QueryMsg::Approvals {
            token_id,
            include_expired,
        } => {
            let res = query_approvals(deps, &env, token_id, include_expired.unwrap_or(false))?;
            Ok(to_json_binary(&res)?)
        }
        _ => ADOContract::default().query(deps, env, msg),
    }
}

pub fn query_transfer_agreement(
    deps: Deps,
    token_id: String,
) -> Result<Option<TransferAgreement>, ContractError> {
    Ok(TRANSFER_AGREEMENTS.may_load(deps.storage, &token_id)?)
}

pub fn query_minter(deps: Deps) -> Result<Addr, ContractError> {
    let minter = query_minter_ownership(deps.storage)?
        .owner
        .unwrap_or(Addr::unchecked(""));

    Ok(minter)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
}
