use std::any::TypeId;

use andromeda_ibc::message_bridge::{ExecuteMsg, IbcExecuteMsg, InstantiateMsg, QueryMsg};

use ado_base::ADOContract;
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, from_binary, to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout,
    MessageInfo, Response, StdError, WasmMsg,
};
use cw2::set_contract_version;
// use serde::de::DeserializeOwned;

use crate::state::{read_chains, read_channel, save_channel};

const CONTRACT_NAME: &str = "crates.io:andromeda-message-bridge";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "message-bridge".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: None,
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
    match msg {
        ExecuteMsg::SendMessage {
            chain,
            recipient,
            message,
        } => execute_send_message(deps, env, chain, recipient, message),
        ExecuteMsg::SaveChannel { channel, chain } => {
            execute_save_channel(deps, info, channel, chain)
        }
    }
}

pub fn execute_send_message(
    deps: DepsMut,
    env: Env,
    chain: String,
    recipient: String,
    message: Binary,
) -> Result<Response, ContractError> {
    let channel = read_channel(deps.storage, chain.clone())?;

    Ok(Response::new()
        .add_attribute("method", "execute_send_message")
        .add_attribute("channel", channel.clone())
        .add_attribute("chain", chain)
        // outbound IBC message, where packet is then received on other chain
        .add_message(IbcMsg::SendPacket {
            channel_id: channel,
            data: to_binary(&IbcExecuteMsg::SendMessage { recipient, message })?,
            timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(300)),
        }))
}

pub fn execute_save_channel(
    deps: DepsMut,
    info: MessageInfo,
    channel: String,
    chain: String,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    save_channel(deps.storage, chain.clone(), channel.clone())?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "execute_save_channel"),
        attr("chain", chain),
        attr("channel", channel),
    ]))
}

/// called on IBC packet receive in other chain
pub fn try_wasm_msg(_deps: DepsMut, target: String, message: Binary) -> Result<WasmMsg, StdError>
// where
//     T: DeserializeOwned,
{
    let _unpacked_message = from_binary(&message)?;
    //TODO change String to AMPPkt once it's been merged with AMP
    let wasm_msg = if TypeId::of::<i32>() == TypeId::of::<String>() {
        WasmMsg::Execute {
            contract_addr: "kernel_address".to_owned(),
            msg: message,
            funds: vec![],
        }
    } else {
        WasmMsg::Execute {
            contract_addr: target,
            msg: message,
            funds: vec![],
        }
    };
    Ok(wasm_msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::ChannelID { chain } => encode_binary(&query_channel_id(deps, chain)?),
        QueryMsg::SupportedChains {} => encode_binary(&query_supported_chains(deps)?),
    }
}

fn query_channel_id(deps: Deps, chain: String) -> Result<String, ContractError> {
    let channel_id = read_channel(deps.storage, chain)?;
    Ok(channel_id)
}

fn query_supported_chains(deps: Deps) -> Result<Vec<String>, ContractError> {
    let chains = read_chains(deps.storage)?;
    Ok(chains)
}
