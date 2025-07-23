use crate::state::{
    FACTORY_DENOMS, LOCKED, MINT_RECIPIENT_AMOUNT, OSMOSIS_MSG_BURN_ID, OSMOSIS_MSG_CREATE_DENOM_ID,
};
use andromeda_socket::osmosis_token_factory::{
    AllLockedResponse, ExecuteMsg, FactoryDenomResponse, InstantiateMsg, LockedInfo,
    LockedResponse, QueryMsg, ReceiveHook,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{addresses::get_raw_address_or_default, AndrAddr, Recipient},
    andr_execute_fn,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, from_json, to_json_binary, wasm_execute, Addr, Binary, CosmosMsg, Deps, DepsMut,
    Env, MessageInfo, QueryRequest, Reply, Response, StdError, SubMsg, SubMsgResponse,
    SubMsgResult, Uint128, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, TokenInfoResponse};
use cw_utils::one_coin;
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as OsmosisCoin,
    osmosis::tokenfactory::v1beta1::{
        MsgBurn, MsgCreateDenom, MsgCreateDenomResponse, MsgMint,
        QueryDenomAuthorityMetadataResponse, TokenfactoryQuerier,
    },
};

const CONTRACT_NAME: &str = "crates.io:andromeda-osmosis-token-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address.clone(),
            owner: msg.owner,
        },
    )?;

    Ok(inst_resp
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateDenom {
            subdenom,
            amount,
            recipient,
        } => execute_create_denom(ctx, subdenom, amount, recipient),
        ExecuteMsg::Receive { msg } => execute_receive(ctx, msg),
        ExecuteMsg::Unlock {
            cw20_addr,
            factory_denom,
            recipient,
        } => execute_unlock(ctx, cw20_addr, factory_denom, recipient),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_receive(ctx: ExecuteContext, msg: Cw20ReceiveMsg) -> Result<Response, ContractError> {
    let hook: ReceiveHook = from_json(&msg.msg)?;

    match hook {
        ReceiveHook::Lock {} => {
            let cw20_addr = ctx.info.sender.clone();
            let user_addr = ctx.deps.api.addr_validate(&msg.sender)?;
            let amount = msg.amount;

            execute_lock(ctx, user_addr, cw20_addr, amount)
        }
    }
}

fn execute_lock(
    ctx: ExecuteContext,
    user_addr: Addr,
    cw20_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Update locked amount for this (user, cw20_token) pair
    LOCKED.update(
        ctx.deps.storage,
        cw20_addr.clone(),
        |existing| -> Result<Uint128, ContractError> { Ok(existing.unwrap_or_default() + amount) },
    )?;

    // Check if factory denom exists for this CW20
    let factory_denom = FACTORY_DENOMS.may_load(ctx.deps.storage, cw20_addr.clone())?;
    match factory_denom {
        Some(denom) => {
            // Denom exists, mint directly
            execute_mint(
                ctx,
                OsmosisCoin {
                    denom,
                    amount: amount.to_string(),
                },
                Some(user_addr.into()),
            )
        }
        None => {
            let token_info: TokenInfoResponse =
                ctx.deps
                    .querier
                    .query(&QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: cw20_addr.to_string(),
                        msg: to_json_binary(&Cw20QueryMsg::TokenInfo {})?,
                    }))?;

            // Create new denom first(mints the token in the reply)
            execute_create_denom(
                ctx,
                token_info.name.to_lowercase(),
                amount,
                Some(user_addr.into()),
            )
        }
    }
}

fn execute_unlock(
    ctx: ExecuteContext,
    cw20_addr: Addr,
    factory_denom: String,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let user_addr = ctx.info.sender.clone();
    let funds = one_coin(&ctx.info)?;

    // 1. Validate factory denom matches the CW20
    let expected_denom = FACTORY_DENOMS.load(ctx.deps.storage, cw20_addr.clone())?;
    ensure!(
        expected_denom == factory_denom,
        ContractError::Std(StdError::generic_err(
            "Invalid factory denom for this CW20 token"
        ))
    );

    // 2. Check that user has enough locked tokens (only original locker can unlock)
    let locked_amount = LOCKED.load(ctx.deps.storage, cw20_addr.clone())?;
    ensure!(
        locked_amount >= funds.amount,
        ContractError::InsufficientFunds {}
    );

    // 3. Prepare CW20 transfer or send message depending if a binary message was provided by the user (sends CW20 tokens back to user or recipient)
    let cw20_exec_sub_msg = if let Some(recipient) = recipient {
        recipient.generate_msg_cw20(
            &ctx.deps.as_ref(),
            Cw20Coin {
                address: cw20_addr.to_string(),
                amount: funds.amount,
            },
        )?
    } else {
        let transfer_msg = Cw20ExecuteMsg::Transfer {
            recipient: user_addr.to_string(),
            amount: funds.amount,
        };
        SubMsg::new(wasm_execute(
            cw20_addr.to_string(),
            &to_json_binary(&transfer_msg)?,
            vec![],
        )?)
    };

    // 4. Update LOCKED state (before preparing burn message)
    LOCKED.save(
        ctx.deps.storage,
        cw20_addr.clone(),
        &locked_amount.checked_sub(funds.amount)?,
    )?;

    // 5. Prepare burn message (burns factory tokens from caller's address)
    let burn_msg = execute_burn(
        ctx,
        OsmosisCoin {
            denom: factory_denom.clone(),
            amount: funds.amount.to_string(),
        },
    )?;

    Ok(Response::new()
        .add_submessage(cw20_exec_sub_msg)
        .add_submessages(burn_msg.messages)
        .add_attributes(burn_msg.attributes)
        .add_attribute("action", "unlock")
        .add_attribute("user", user_addr.to_string())
        .add_attribute("cw20_addr", cw20_addr.to_string())
        .add_attribute("factory_denom", factory_denom)
        .add_attribute("amount", funds.amount.to_string()))
}

fn execute_create_denom(
    ctx: ExecuteContext,
    subdenom: String,
    amount: Uint128,
    // Defaults to message sender
    recipient: Option<AndrAddr>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;

    let msg = MsgCreateDenom {
        sender: env.contract.address.to_string(),
        subdenom,
    };
    // Initiates minting of the denom in the Reply
    let sub_msg = SubMsg::reply_always(msg, OSMOSIS_MSG_CREATE_DENOM_ID);

    let recipient =
        get_raw_address_or_default(&deps.as_ref(), &recipient, info.sender.as_str())?.into_string();

    MINT_RECIPIENT_AMOUNT.save(deps.storage, &(recipient, amount))?;

    Ok(Response::default().add_submessage(sub_msg))
}

fn execute_mint(
    ctx: ExecuteContext,
    coin: OsmosisCoin,
    // Defaults to message sender
    recipient: Option<AndrAddr>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;

    let recipient =
        get_raw_address_or_default(&deps.as_ref(), &recipient, info.sender.as_str())?.into_string();

    let msg = MsgMint {
        sender: env.contract.address.to_string(),
        amount: Some(coin),
        mint_to_address: recipient,
    };
    Ok(Response::default().add_message(msg))
}

// TODO: https://github.com/andromedaprotocol/andromeda-core/pull/929#discussion_r2207821091
fn execute_burn(ctx: ExecuteContext, coin: OsmosisCoin) -> Result<Response, ContractError> {
    let ExecuteContext { env, info, .. } = ctx;

    let msg = MsgBurn {
        sender: env.contract.address.to_string(),
        amount: Some(coin),
        burn_from_address: info.sender.to_string(), // Always burn from caller
    };
    let sub_msg = SubMsg::reply_always(msg, OSMOSIS_MSG_BURN_ID);

    Ok(Response::default().add_submessage(sub_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::TokenAuthority { denom } => {
            let res: QueryDenomAuthorityMetadataResponse =
                TokenfactoryQuerier::new(&deps.querier).denom_authority_metadata(denom)?;
            encode_binary(&res)
        }
        QueryMsg::Locked { cw20_addr } => encode_binary(&query_locked(deps, cw20_addr)?),
        QueryMsg::FactoryDenom { cw20_addr } => {
            encode_binary(&query_factory_denom(deps, cw20_addr)?)
        }
        QueryMsg::AllLocked {} => encode_binary(&query_all_locked(deps)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, env, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        OSMOSIS_MSG_CREATE_DENOM_ID => {
            #[allow(deprecated)]
            if let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result {
                let res: MsgCreateDenomResponse = b.try_into().map_err(ContractError::Std)?;
                let (recipient, amount) = MINT_RECIPIENT_AMOUNT.load(deps.storage)?;
                MINT_RECIPIENT_AMOUNT.remove(deps.storage);

                let msg = MsgMint {
                    sender: env.contract.address.to_string(),
                    mint_to_address: recipient,
                    amount: Some(OsmosisCoin {
                        denom: res.new_token_denom,
                        amount: amount.to_string(),
                    }),
                };
                let mint_msg: CosmosMsg = msg.into();
                Ok(Response::default().add_message(mint_msg))
            } else {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis denom creation failed with error: {:?}",
                    msg.result.unwrap_err()
                ))))
            }
        }

        OSMOSIS_MSG_BURN_ID => {
            // Send IBC packet to unlock the cw20
            if msg.result.is_err() {
                Err(ContractError::Std(StdError::generic_err(format!(
                    "Osmosis swap failed with error: {:?}",
                    msg.result.unwrap_err()
                ))))
            } else {
                Ok(Response::default().add_attributes(vec![attr("action", "token_burned")]))
            }
        }
        _ => Err(ContractError::Std(StdError::generic_err(
            "Invalid Reply ID".to_string(),
        ))),
    }
}

fn query_locked(deps: Deps, cw20_addr: Addr) -> Result<LockedResponse, ContractError> {
    let amount = LOCKED
        .may_load(deps.storage, cw20_addr)?
        .unwrap_or_default();
    Ok(LockedResponse { amount })
}

fn query_factory_denom(deps: Deps, cw20_addr: Addr) -> Result<FactoryDenomResponse, ContractError> {
    let denom = FACTORY_DENOMS.may_load(deps.storage, cw20_addr)?;
    Ok(FactoryDenomResponse { denom })
}

fn query_all_locked(deps: Deps) -> Result<AllLockedResponse, ContractError> {
    let locked: Vec<LockedInfo> = LOCKED
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            let (cw20_addr, amount) = item.ok()?;
            if !amount.is_zero() {
                Some(LockedInfo { cw20_addr, amount })
            } else {
                None
            }
        })
        .collect();
    Ok(AllLockedResponse { locked })
}
