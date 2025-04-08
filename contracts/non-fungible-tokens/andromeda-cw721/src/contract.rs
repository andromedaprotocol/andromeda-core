use andromeda_std::andr_execute_fn;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, from_json, has_coins, to_json_binary, Addr, Api, BankMsg, Binary, Coin,
    CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, QuerierWrapper, Reply, Response, StdError,
    SubMsg, Uint128,
};
use cw721::error::Cw721ContractError;
use cw721::msg::{Cw721InstantiateMsg, OwnerOfResponse};
use cw721::Approval;

use crate::state::{is_archived, ANDR_MINTER, ARCHIVED, TRANSFER_AGREEMENTS};
use andromeda_non_fungible_tokens::cw721::{
    AndrCW721Contract, BatchSendMsg, ExecuteMsg, InstantiateMsg, MintMsg, QueryMsg,
    TransferAgreement,
};
use andromeda_std::common::rates::get_tax_amount;
use andromeda_std::{
    ado_base::AndromedaQuery,
    ado_contract::{permissioning::is_context_permissioned, ADOContract},
    amp::AndrAddr,
    common::context::ExecuteContext,
};

use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    common::Funds,
    error::ContractError,
};
use cw721::state::{Cw721Config, NftInfo};
use cw721::traits::{Cw721Execute, Cw721Query};
use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;

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
    let cw721_instantiate_msg: Cw721InstantiateMsg<Empty> = Cw721InstantiateMsg {
        name: msg.name,
        symbol: msg.symbol,
        minter: Some(msg.minter.to_string()),
        collection_info_extension: Empty::default(),
        creator: None,
        withdraw_address: None,
    };

    let cw721_contract = AndrCW721Contract;
    let res = cw721_contract.instantiate(deps.branch(), &env, &info, cw721_instantiate_msg)?;

    let contract = ADOContract::default();
    ANDR_MINTER.save(deps.storage, &msg.minter)?;

    contract.permission_action(deps.storage, MINT_ACTION)?;

    let resp = contract.instantiate(
        deps.storage,
        env,
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

    Ok(res
        .add_attributes(vec![attr("minter", msg.minter)])
        .add_submessages(resp.messages))
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
        // Attempt to match the message as a cw721 message first, if it fails, fallback to the
        // default ADO execute function.
        _ => match msg.clone().try_into() {
            Ok(cw721_msg) => execute_cw721(ctx, cw721_msg),
            Err(_) => ADOContract::default().execute(ctx, msg),
        },
    }?;
    Ok(res)
}

fn execute_cw721(ctx: ExecuteContext, msg: Cw721ExecuteMsg) -> Result<Response, ContractError> {
    let contract = AndrCW721Contract;

    Ok(contract.execute(ctx.deps, &ctx.env, &ctx.info, msg)?)
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
    mint(ctx, token_id, token_uri, owner)
}

fn mint(
    ctx: ExecuteContext,
    token_id: String,
    token_uri: Option<String>,
    owner: AndrAddr,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    let owner_addr = owner.get_raw_address(&deps.as_ref())?;

    let token = NftInfo {
        owner: owner_addr.clone(),
        approvals: vec![],
        token_uri: token_uri.clone(),
        extension: Empty::default(),
    };

    let cw721_config = Cw721Config::<Empty>::default();
    cw721_config
        .nft_info
        .update(deps.storage, &token_id, |old| match old {
            Some(_) => Err(Cw721ContractError::Claimed {}),
            None => Ok(token),
        })?;

    cw721_config.increment_tokens(deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("minter", info.sender)
        .add_attribute("owner", owner)
        .add_attribute("token_id", token_id))
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
        let mint_resp = mint(ctx, msg.token_id, msg.token_uri, msg.owner)?;
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
        contract: base_contract,
        ..
    } = ctx;
    // Reduce all responses into one.
    let mut resp = Response::new();
    let recipient_address = recipient.get_raw_address(&deps.as_ref())?.into_string();
    let contract = AndrCW721Contract;

    let OwnerOfResponse { owner, .. } =
        contract.query_owner_of(deps.as_ref(), &env, token_id.clone(), false)?;
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
                let msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
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

    let approvals = contract.query_approvals(deps.as_ref(), &env, token_id.clone(), true)?;
    let operators =
        contract.query_operators(deps.as_ref(), &env, owner.clone(), true, None, None)?;
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
    let response = contract.transfer_nft(
        deps.branch(),
        &env,
        &transfer_info,
        recipient.to_string(),
        token_id.clone(),
    )?;
    TRANSFER_AGREEMENTS.remove(deps.storage, &token_id);

    // Extract elements from the response and include them in the final response
    let mut response = response;
    for attr in response.attributes.clone() {
        resp = resp.add_attribute(attr.key, attr.value);
    }
    for event in response.events.clone() {
        resp = resp.add_event(event);
    }
    for submsg in response.messages {
        resp = resp.add_submessage(submsg);
    }
    response = resp;
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
    let contract = AndrCW721Contract;
    let token_owner = contract.query_owner_of(deps.as_ref(), &ctx.env, token_id.clone(), false)?;
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
            contract.approve(
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
        contract.revoke_all(deps, &ctx.env, &ctx.info, token_id.clone())?;
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
    let contract = AndrCW721Contract;
    let token_owner = contract.query_owner_of(deps.as_ref(), &ctx.env, token_id.clone(), false)?;
    ensure!(
        token_owner.owner == info.sender.as_ref(),
        ContractError::Unauthorized {}
    );

    ARCHIVED.save(deps.storage, &token_id, &true)?;

    // TODO should we call contract in this function?
    // contract.tokens.save(deps.storage, &token_id, &token)?;

    Ok(Response::default())
}

fn execute_burn(ctx: ExecuteContext, token_id: String) -> Result<Response, ContractError> {
    let ExecuteContext { deps, ref info, .. } = ctx;
    let contract = AndrCW721Contract;
    // let token = contract.tokens.load(deps.storage, &token_id)?;
    let token_owner = contract.query_owner_of(deps.as_ref(), &ctx.env, token_id.clone(), false)?;
    ensure!(
        token_owner.owner == info.sender.as_ref(),
        ContractError::Unauthorized {}
    );
    ensure!(
        !is_archived(deps.storage, &token_id)?.is_archived,
        ContractError::TokenIsArchived {}
    );

    contract.burn_nft(deps, &ctx.env, &ctx.info, token_id.clone())?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "burn"),
        attr("token_id", token_id),
        attr("sender", info.sender.as_str()),
    ]))
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
    let contract = AndrCW721Contract;
    TRANSFER_AGREEMENTS.remove(deps.storage, &token_id);
    let contract_addr = contract_addr.get_raw_address(&deps.as_ref())?.into_string();

    Ok(contract.send_nft(deps, &env, &info, contract_addr, token_id, msg)?)
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
        let send_resp = execute_send_nft(ctx, item.token_id, item.contract_addr, item.msg)?;
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
            Ok(to_json_binary(&is_archived(deps.storage, &token_id)?)?)
        }
        QueryMsg::TransferAgreement { token_id } => {
            Ok(to_json_binary(&query_transfer_agreement(deps, token_id)?)?)
        }
        QueryMsg::Minter {} => Ok(to_json_binary(&query_minter(deps)?)?),
        _ => {
            let serialized = to_json_binary(&msg)?;
            match from_json::<AndromedaQuery>(&serialized) {
                Ok(msg) => ADOContract::default().query(deps, env, msg),
                _ => Ok(AndrCW721Contract.query(deps, &env, msg.into())?),
            }
        }
    }
}

pub fn query_transfer_agreement(
    deps: Deps,
    token_id: String,
) -> Result<Option<TransferAgreement>, ContractError> {
    Ok(TRANSFER_AGREEMENTS.may_load(deps.storage, &token_id)?)
}

pub fn query_minter(deps: Deps) -> Result<Addr, ContractError> {
    let minter = ANDR_MINTER.load(deps.storage)?;
    minter.get_raw_address(&deps)
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
