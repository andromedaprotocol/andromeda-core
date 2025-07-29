use crate::state::{DENOMS_TO_OWNER, FACTORY_DENOMS, LOCKED, OSMOSIS_MSG_BURN_ID};
use andromeda_socket::osmosis_token_factory::{
    get_factory_denom, is_cw20_contract, AllLockedResponse, Cw20HookMsg, ExecuteMsg,
    FactoryDenomResponse, InstantiateMsg, LockedInfo, LockedResponse, QueryMsg,
};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{addresses::get_raw_address_or_default, AndrAddr, Recipient},
    andr_execute_fn,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, from_json, to_json_binary, wasm_execute, Addr, Binary, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Reply, Response, StdError, SubMsg, Uint128, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, TokenInfoResponse};
use cw_utils::one_coin;
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as OsmosisCoin,
    osmosis::tokenfactory::v1beta1::{
        MsgBurn, MsgCreateDenom, MsgMint, QueryDenomAuthorityMetadataResponse, TokenfactoryQuerier,
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
    println!("reached here1");
    match msg {
        ExecuteMsg::CreateDenom { subdenom } => execute_create_denom_direct(ctx, subdenom),
        ExecuteMsg::Mint {
            recipient,
            subdenom,
            amount,
        } => {
            let factory_denom = get_factory_denom(&ctx.env, &subdenom);
            let denom_owner = DENOMS_TO_OWNER.load(ctx.deps.storage, factory_denom.clone())?;
            ensure!(
                denom_owner == ctx.info.sender,
                ContractError::InvalidFunds {
                    msg: format!("Invalid cw20, the authorized one is {}", denom_owner),
                }
            );

            execute_mint(
                ctx,
                OsmosisCoin {
                    denom: factory_denom,
                    amount: amount.to_string(),
                },
                recipient,
            )
        }
        ExecuteMsg::Burn {} => {
            let funds = one_coin(&ctx.info)?;
            let denom_owner = DENOMS_TO_OWNER.load(ctx.deps.storage, funds.denom.clone())?;
            // Check if the denom owner is a cw20 contract
            let is_cw20 = is_cw20_contract(&ctx.deps.querier, denom_owner.as_str())?;
            ensure!(
                !is_cw20,
                ContractError::InvalidFunds {
                    msg: "Tokens created from cw20 should be burned using the `Unlock` message"
                        .to_string(),
                }
            );
            execute_burn(
                ctx,
                OsmosisCoin {
                    denom: funds.denom,
                    amount: funds.amount.to_string(),
                },
            )
        }
        ExecuteMsg::Unlock { recipient } => execute_unlock(ctx, recipient),
        ExecuteMsg::Receive(msg) => execute_receive(ctx, msg),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_receive(
    ctx: ExecuteContext,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    println!("reached here2");

    match from_json(&receive_msg.msg)? {
        Cw20HookMsg::Lock { recipient } => {
            let cw20_addr = ctx.info.sender.clone();
            let user_addr = ctx.deps.api.addr_validate(&receive_msg.sender)?;
            let amount = receive_msg.amount;
            let recipient = recipient
                .map(|r| r.get_raw_address(&ctx.deps.as_ref()))
                .transpose()?
                .unwrap_or(user_addr);

            execute_lock(ctx, recipient, cw20_addr, amount)
        }
    }
}

fn execute_lock(
    ctx: ExecuteContext,
    user_addr: Addr,
    cw20_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Update locked amount for this cw20 address
    LOCKED.update(
        ctx.deps.storage,
        cw20_addr.clone(),
        |existing| -> Result<Uint128, ContractError> { Ok(existing.unwrap_or_default() + amount) },
    )?;

    // Check if factory denom exists for this CW20
    let factory_denom = FACTORY_DENOMS.may_load(ctx.deps.storage, cw20_addr.clone())?;
    match factory_denom {
        Some(denom) => {
            let denom_owner = DENOMS_TO_OWNER.load(ctx.deps.storage, denom.clone())?;
            ensure!(
                denom_owner == cw20_addr,
                ContractError::InvalidFunds {
                    msg: format!("Invalid cw20, the authorized one is {}", denom_owner)
                }
            );
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

            let subdenom = token_info.symbol.to_lowercase();
            let new_denom = get_factory_denom(&ctx.env, &subdenom);
            // If the create denom function fails due duplicate denom, the state will revert
            DENOMS_TO_OWNER.save(ctx.deps.storage, new_denom.clone(), &cw20_addr)?;

            FACTORY_DENOMS.save(ctx.deps.storage, cw20_addr.clone(), &new_denom)?;

            // Create new denom first and then mints the tokens
            execute_create_denom_and_mint(ctx, subdenom, amount, Some(user_addr.into()))
        }
    }
}

fn execute_unlock(
    ctx: ExecuteContext,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let user_addr = ctx.info.sender.clone();
    let funds = one_coin(&ctx.info)?;
    let factory_denom = funds.denom;

    let denom_owner = DENOMS_TO_OWNER.load(ctx.deps.storage, factory_denom.clone())?;

    // 1. Validate factory denom matches the CW20
    let expected_denom = FACTORY_DENOMS.load(ctx.deps.storage, denom_owner.clone())?;
    ensure!(
        expected_denom == factory_denom,
        ContractError::Std(StdError::generic_err(
            "Invalid factory denom for this CW20 token"
        ))
    );

    // 2. Check that there are enough locked tokens (only original locker can unlock)
    let locked_amount = LOCKED.load(ctx.deps.storage, denom_owner.clone())?;
    ensure!(
        locked_amount >= funds.amount,
        ContractError::InsufficientFunds {}
    );

    // 3. Prepare CW20 transfer or send message depending if a binary message was provided by the user (sends CW20 tokens back to user or recipient)
    let cw20_exec_sub_msg = if let Some(recipient) = recipient {
        recipient.generate_msg_cw20(
            &ctx.deps.as_ref(),
            Cw20Coin {
                address: denom_owner.to_string(),
                amount: funds.amount,
            },
        )?
    } else {
        let transfer_msg = Cw20ExecuteMsg::Transfer {
            recipient: user_addr.to_string(),
            amount: funds.amount,
        };
        SubMsg::new(wasm_execute(
            denom_owner.to_string(),
            &to_json_binary(&transfer_msg)?,
            vec![],
        )?)
    };

    // 4. Update LOCKED state (before preparing burn message)
    LOCKED.save(
        ctx.deps.storage,
        denom_owner.clone(),
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
        .add_attribute("cw20_addr", denom_owner.to_string())
        .add_attribute("factory_denom", factory_denom)
        .add_attribute("amount", funds.amount.to_string()))
}

// Used for cross-chain creation of denom
fn execute_create_denom_direct(
    ctx: ExecuteContext,
    subdenom: String,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, env, info, ..
    } = ctx;

    let new_denom = get_factory_denom(&env, &subdenom);
    let denom_owner = DENOMS_TO_OWNER.may_load(deps.storage, new_denom.clone())?;
    ensure!(
        denom_owner.is_none(),
        ContractError::InvalidFunds {
            msg: "Denom already exists".to_string(),
        }
    );

    DENOMS_TO_OWNER.save(deps.storage, new_denom.clone(), &info.sender)?;
    println!("create denom direct");
    let create_denom_msg = SubMsg::new(MsgCreateDenom {
        sender: env.contract.address.to_string(),
        subdenom: subdenom.clone(),
    });

    Ok(Response::default()
        .add_submessages(vec![create_denom_msg])
        .add_attribute("action", "create_denom_direct")
        .add_attribute("subdenom", subdenom)
        .add_attribute("owner", info.sender.to_string()))
}

fn execute_create_denom_and_mint(
    ctx: ExecuteContext,
    subdenom: String,
    amount: Uint128,
    // Defaults to message sender
    recipient: Option<AndrAddr>,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, env, .. } = ctx;

    let create_denom_msg = SubMsg::new(MsgCreateDenom {
        sender: env.contract.address.to_string(),
        subdenom: subdenom.clone(),
    });

    let new_denom = get_factory_denom(&env, &subdenom);
    let recipient = get_raw_address_or_default(&deps.as_ref(), &recipient, &new_denom)?;

    let mint_msg = SubMsg::new(MsgMint {
        sender: env.contract.address.to_string(),
        mint_to_address: recipient.clone().into_string(),
        amount: Some(OsmosisCoin {
            denom: new_denom.clone(),
            amount: amount.to_string(),
        }),
    });

    Ok(Response::default()
        .add_submessages(vec![create_denom_msg, mint_msg])
        .add_attribute("action", "create_denom_and_mint")
        .add_attribute("denom", new_denom)
        .add_attribute("amount", amount.to_string())
        .add_attribute("recipient", recipient.into_string()))
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
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
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
