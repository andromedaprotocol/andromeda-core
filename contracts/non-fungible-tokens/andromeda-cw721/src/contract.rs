#[cfg(not(feature = "imported"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, from_binary, has_coins, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Deps,
    DepsMut, Empty, Env, MessageInfo, QuerierWrapper, Response, SubMsg, Uint128,
};

use crate::state::{
    is_archived, ANDR_MINTER, ARCHIVED, BATCH_MINT_ACTION, MINT_ACTION, TRANSFER_AGREEMENTS,
};
use andromeda_non_fungible_tokens::cw721::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, MintMsg, QueryMsg, TokenExtension, TransferAgreement,
};
use andromeda_std::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    ado_contract::{permissioning::is_context_permissioned_strict, ADOContract},
    common::{actions::call_action, context::ExecuteContext},
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg},
    common::encode_binary,
    common::rates::get_tax_amount,
    common::Funds,
    error::{from_semver, ContractError},
};
use cw721::{ContractInfoResponse, Cw721Execute};
use cw721_base::{state::TokenInfo, Cw721Contract, ExecuteMsg as Cw721ExecuteMsg};

pub type AndrCW721Contract<'a> = Cw721Contract<'a, TokenExtension, Empty, ExecuteMsg, QueryMsg>;
const CONTRACT_NAME: &str = "crates.io:andromeda-cw721";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let contract_info = ContractInfoResponse {
        name: msg.name,
        symbol: msg.symbol,
    };

    // Do this directly instead of with cw721_contract.instantiate because we want to have minter
    // be an AndrAddress, which cannot be validated right away.
    AndrCW721Contract::default()
        .contract_info
        .save(deps.storage, &contract_info)?;

    let contract = ADOContract::default();
    ANDR_MINTER.save(deps.storage, &msg.minter)?;

    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: "cw721".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    let modules_resp =
        contract.register_modules(info.sender.as_str(), deps.storage, msg.modules)?;

    Ok(resp
        .add_submessages(modules_resp.messages)
        .add_attributes(modules_resp.attributes)
        .add_attributes(vec![attr("minter", msg.minter)]))
}

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    if let ExecuteMsg::AMPReceive(pkt) = msg {
        ADOContract::default().execute_amp_receive(
            ExecuteContext::new(deps, info, env),
            pkt,
            handle_execute,
        )
    } else {
        let ctx = ExecuteContext::new(deps, info, env);
        handle_execute(ctx, msg)
    }
}

fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;
    let payee = if let Some(amp_ctx) = ctx.amp_ctx.clone() {
        ctx.deps
            .api
            .addr_validate(amp_ctx.ctx.get_origin().as_str())?
    } else {
        ctx.info.sender.clone()
    };

    let fee_msg = ADOContract::default().pay_fee(
        ctx.deps.storage,
        &ctx.deps.querier,
        msg.as_ref().to_string(),
        payee,
    )?;

    if let ExecuteMsg::Approve { token_id, .. } = &msg {
        ensure!(
            !is_archived(ctx.deps.storage, token_id)?,
            ContractError::TokenIsArchived {}
        );
    }

    let res = match msg {
        ExecuteMsg::Mint {
            token_id,
            token_uri,
            owner,
            extension,
        } => execute_mint(ctx, token_id, token_uri, owner, extension),
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
        _ => {
            let serialized = to_binary(&msg)?;

            match from_binary::<AndromedaMsg>(&serialized) {
                Ok(msg) => contract.execute(ctx, msg),
                Err(_) => execute_cw721(ctx, msg.into()),
            }
        }
    }?;
    Ok(res.add_submessage(fee_msg))
}

fn execute_cw721(
    ctx: ExecuteContext,
    msg: Cw721ExecuteMsg<TokenExtension, ExecuteMsg>,
) -> Result<Response, ContractError> {
    let contract = AndrCW721Contract::default();
    Ok(contract.execute(ctx.deps, ctx.env, ctx.info, msg)?)
}

fn execute_mint(
    ctx: ExecuteContext,
    token_id: String,
    token_uri: Option<String>,
    owner: String,
    extension: TokenExtension,
) -> Result<Response, ContractError> {
    let minter = ANDR_MINTER
        .load(ctx.deps.storage)?
        .get_raw_address(&ctx.deps.as_ref())?;
    ensure!(
        ctx.contains_sender(minter.as_str())
            | is_context_permissioned_strict(
                ctx.deps.storage,
                &ctx.info,
                &ctx.env,
                &ctx.amp_ctx,
                MINT_ACTION
            )?,
        ContractError::Unauthorized {}
    );
    mint(ctx, token_id, token_uri, owner, extension)
}

fn mint(
    ctx: ExecuteContext,
    token_id: String,
    token_uri: Option<String>,
    owner: String,
    extension: TokenExtension,
) -> Result<Response, ContractError> {
    let cw721_contract = AndrCW721Contract::default();
    let token = TokenInfo {
        owner: ctx.deps.api.addr_validate(&owner)?,
        approvals: vec![],
        token_uri,
        extension,
    };

    cw721_contract
        .tokens
        .update(ctx.deps.storage, &token_id, |old| match old {
            Some(_) => Err(ContractError::Claimed {}),
            None => Ok(token),
        })?;

    cw721_contract.increment_tokens(ctx.deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("minter", ctx.info.sender)
        .add_attribute("owner", owner)
        .add_attribute("token_id", token_id))
}

fn execute_batch_mint(
    mut ctx: ExecuteContext,
    tokens_to_mint: Vec<MintMsg>,
) -> Result<Response, ContractError> {
    let mut resp = Response::default();
    let minter = ANDR_MINTER
        .load(ctx.deps.storage)?
        .get_raw_address(&ctx.deps.as_ref())?;
    ensure!(
        ctx.contains_sender(minter.as_str())
            | is_context_permissioned_strict(
                ctx.deps.storage,
                &ctx.info,
                &ctx.env,
                &ctx.amp_ctx,
                BATCH_MINT_ACTION
            )?,
        ContractError::Unauthorized {}
    );
    ensure!(
        !tokens_to_mint.is_empty(),
        ContractError::Std(cosmwasm_std::StdError::GenericErr {
            msg: String::from("No tokens to mint")
        })
    );
    for msg in tokens_to_mint {
        let ctx = ExecuteContext {
            deps: ctx.deps.branch(),
            info: ctx.info.clone(),
            env: ctx.env.clone(),
            amp_ctx: ctx.amp_ctx.clone(),
        };
        let mint_resp = mint(ctx, msg.token_id, msg.token_uri, msg.owner, msg.extension)?;
        resp = resp
            .add_attributes(mint_resp.attributes)
            .add_submessages(mint_resp.messages);
    }

    Ok(resp)
}

fn execute_transfer(
    env: ExecuteContext,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = env;
    let base_contract = ADOContract::default();
    let responses = base_contract.module_hook::<Response>(
        &deps.as_ref(),
        AndromedaHook::OnTokenTransfer {
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
    ensure!(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {}
    );

    let tax_amount = if let Some(agreement) =
        &TRANSFER_AGREEMENTS.may_load(deps.storage, &token_id)?
    {
        let agreement_amount = get_transfer_agreement_amount(deps.api, &deps.querier, agreement)?;
        let (mut msgs, events, remainder) = base_contract.on_funds_transfer(
            &deps.as_ref(),
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

    check_can_send(deps.as_ref(), env, info, &token_id, &token, tax_amount)?;
    token.owner = deps.api.addr_validate(&recipient)?;
    token.approvals.clear();
    TRANSFER_AGREEMENTS.remove(deps.storage, &token_id);
    contract.tokens.save(deps.storage, &token_id, &token)?;
    Ok(resp
        .add_attribute("action", "transfer")
        .add_attribute("recipient", recipient))
}

fn get_transfer_agreement_amount(
    _api: &dyn Api,
    _querier: &QuerierWrapper,
    agreement: &TransferAgreement,
) -> Result<Coin, ContractError> {
    let agreement_amount = agreement.amount.clone();
    Ok(agreement_amount)
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
    env: ExecuteContext,
    token_id: String,
    agreement: Option<TransferAgreement>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = env;
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(deps.storage, &token_id)?;
    ensure!(token.owner == info.sender, ContractError::Unauthorized {});
    ensure!(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {}
    );
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

fn execute_archive(env: ExecuteContext, token_id: String) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = env;
    ensure!(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {}
    );
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(deps.storage, &token_id)?;
    ensure!(token.owner == info.sender, ContractError::Unauthorized {});

    ARCHIVED.save(deps.storage, &token_id, &true)?;

    contract.tokens.save(deps.storage, &token_id, &token)?;

    Ok(Response::default())
}

fn execute_burn(env: ExecuteContext, token_id: String) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = env;
    let contract = AndrCW721Contract::default();
    let token = contract.tokens.load(deps.storage, &token_id)?;
    ensure!(token.owner == info.sender, ContractError::Unauthorized {});
    ensure!(
        !is_archived(deps.storage, &token_id)?,
        ContractError::TokenIsArchived {}
    );

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

fn execute_send_nft(
    ctx: ExecuteContext,
    token_id: String,
    contract_addr: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    let contract = AndrCW721Contract::default();
    TRANSFER_AGREEMENTS.remove(deps.storage, &token_id);

    Ok(contract.send_nft(deps, env, info, contract_addr, token_id, msg)?)
}

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::IsArchived { token_id } => Ok(to_binary(&is_archived(deps.storage, &token_id)?)?),
        QueryMsg::TransferAgreement { token_id } => {
            Ok(to_binary(&query_transfer_agreement(deps, token_id)?)?)
        }
        QueryMsg::Minter {} => Ok(to_binary(&query_minter(deps)?)?),
        _ => {
            let serialized = to_binary(&msg)?;
            match from_binary::<AndromedaQuery>(&serialized) {
                Ok(msg) => ADOContract::default().query(deps, env, msg),
                _ => Ok(AndrCW721Contract::default().query(deps, env, msg.into())?),
            }
        } // _ => Ok(AndrCW721Contract::default().query(deps, env, msg.into())?),
    }
}

pub fn query_transfer_agreement(
    deps: Deps,
    token_id: String,
) -> Result<Option<TransferAgreement>, ContractError> {
    Ok(TRANSFER_AGREEMENTS.may_load(deps.storage, &token_id)?)
}

pub fn query_minter(deps: Deps) -> Result<String, ContractError> {
    let owner = ADOContract::default().query_contract_owner(deps)?;
    Ok(owner.owner)
}

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
