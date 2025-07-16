use crate::state::{MINT_RECIPIENT_AMOUNT, OSMOSIS_MSG_BURN_ID, OSMOSIS_MSG_CREATE_DENOM_ID};
use andromeda_socket::osmosis_token_factory::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
    amp::{addresses::get_raw_address_or_default, AndrAddr},
    andr_execute_fn,
    common::{context::ExecuteContext, encode_binary},
    error::ContractError,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, SubMsg,
    SubMsgResponse, SubMsgResult, Uint128,
};
use cw2::set_contract_version;
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
        ExecuteMsg::Mint { coin, recipient } => execute_mint(ctx, coin, recipient),
        ExecuteMsg::Burn { coin } => execute_burn(ctx, coin),
        _ => ADOContract::default().execute(ctx, msg),
    }
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
    let ExecuteContext { env, .. } = ctx;

    let msg = MsgBurn {
        sender: env.contract.address.to_string(),
        amount: Some(coin),
        burn_from_address: env.contract.address.to_string(),
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
