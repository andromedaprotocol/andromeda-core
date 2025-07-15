use andromeda_std::andr_execute_fn;

use andromeda_std::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    ado_contract::ADOContract,
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

use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmosisCoin;
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgCreateDenom, MsgCreateDenomResponse, MsgMint, QueryDenomAuthorityMetadataResponse,
    TokenfactoryQuerier,
};

use crate::state::{
    AUTHORIZED_ADDRESS, MINT_AMOUNT, OSMOSIS_MSG_BURN_ID, OSMOSIS_MSG_CREATE_DENOM_ID,
    OSMOSIS_MSG_MINT_ID,
};

use andromeda_socket::osmosis_token_factory::{ExecuteMsg, InstantiateMsg, QueryMsg};

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
    let authorized_address = msg.authorized_address.get_raw_address(&deps.as_ref())?;
    AUTHORIZED_ADDRESS.save(deps.storage, &authorized_address.into_string())?;

    Ok(inst_resp
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[andr_execute_fn]
pub fn execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateDenom { subdenom, amount } => execute_create_denom(ctx, subdenom, amount),
        ExecuteMsg::Mint { coin } => execute_mint(ctx, coin),
        ExecuteMsg::Burn { coin } => execute_burn(ctx, coin),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_create_denom(
    ctx: ExecuteContext,
    subdenom: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // TODO: this is commented to facilitate initial testing
    // is_authorized(&ctx)?;

    let ExecuteContext { deps, env, .. } = ctx;

    let msg = MsgCreateDenom {
        sender: env.contract.address.to_string(),
        subdenom,
    };
    let mint_msg: CosmosMsg = msg.into();
    // Initiates minting of the denom in the Reply, sets the contract as the owner
    let sub_msg = SubMsg::reply_always(mint_msg, OSMOSIS_MSG_CREATE_DENOM_ID);
    MINT_AMOUNT.save(deps.storage, &amount)?;

    Ok(Response::default().add_submessage(sub_msg))
}

fn execute_mint(ctx: ExecuteContext, coin: OsmosisCoin) -> Result<Response, ContractError> {
    // TODO: this is commented to facilitate initial testing
    // is_authorized(&ctx)?;
    let ExecuteContext { env, .. } = ctx;

    let msg = MsgMint {
        sender: env.contract.address.to_string(),
        amount: Some(coin),
        mint_to_address: env.contract.address.to_string(),
    };
    let mint_msg: CosmosMsg = msg.into();
    let sub_msg = SubMsg::reply_always(mint_msg, OSMOSIS_MSG_MINT_ID);

    Ok(Response::default().add_submessage(sub_msg))
}

fn execute_burn(ctx: ExecuteContext, coin: OsmosisCoin) -> Result<Response, ContractError> {
    // TODO: this is commented to facilitate initial testing
    // is_authorized(&ctx)?;
    let ExecuteContext { env, .. } = ctx;
    let msg = MsgBurn {
        sender: env.contract.address.to_string(),
        amount: Some(coin),
        burn_from_address: env.contract.address.to_string(),
    };
    let burn_msg: CosmosMsg = msg.into();
    let sub_msg = SubMsg::reply_always(burn_msg, OSMOSIS_MSG_BURN_ID);
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
                let amount = MINT_AMOUNT.load(deps.storage)?;
                MINT_AMOUNT.remove(deps.storage);

                let msg = MsgMint {
                    sender: env.contract.address.to_string(),
                    mint_to_address: env.contract.address.to_string(),
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
