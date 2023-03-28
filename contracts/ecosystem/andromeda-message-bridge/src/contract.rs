use ado_base::ADOContract;
use andromeda_ibc::message_bridge::{ExecuteMsg, IbcExecuteMsg, InstantiateMsg, QueryMsg};
use common::{ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response,
    WasmMsg,
};
use cw2::set_contract_version;

use crate::state::{read_chains, read_channel, save_channel};

const CONTRACT_NAME: &str = "crates.io:andromeda-message-bridge";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
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
            kernel_address: msg.kernel_address,
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
        ExecuteMsg::SendAmpPacket { chain, message } => {
            execute_send_amp_packet(deps, env, chain, message)
        }
    }
}

pub fn execute_send_amp_packet(
    deps: DepsMut,
    env: Env,
    chain: String,
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
            data: to_binary(&IbcExecuteMsg::SendAmpPacket { message })?,
            timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(300)),
        }))
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
pub fn try_wasm_msg(
    _deps: DepsMut,
    target: String,
    message: Binary,
) -> Result<WasmMsg, ContractError> {
    Ok(WasmMsg::Execute {
        contract_addr: target,
        msg: message,
        funds: vec![],
    })
}

/// called on IBC packet receive in other chain
pub fn try_wasm_msg_amp(deps: DepsMut, message: Binary) -> Result<WasmMsg, ContractError> {
    // Get kernel address
    let kernel_address = ADOContract::default()
        .get_kernel_address(deps.storage)?
        .to_string();
    Ok(WasmMsg::Execute {
        contract_addr: kernel_address,
        msg: message,
        funds: vec![],
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CHAIN_CHANNELS;
    use andromeda_testing::testing::mock_querier::mock_dependencies_custom;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            kernel_address: Some("kernel_address".to_string()),
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_save_channel() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            kernel_address: Some("kernel_address".to_string()),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
        let msg = ExecuteMsg::SaveChannel {
            channel: "channel-1".to_owned(),
            chain: "juno".to_owned(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::SaveChannel {
            channel: "channel-2".to_owned(),
            chain: "andromeda".to_owned(),
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let saved_channel = CHAIN_CHANNELS
            .load(&deps.storage, "juno".to_string())
            .unwrap();
        assert_eq!(saved_channel, "channel-1".to_string());

        // Query testing
        let query_msg = QueryMsg::SupportedChains {};
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let supported_chains: Vec<String> = from_binary(&res).unwrap();
        let expected_suuported_chains = vec!["juno".to_string(), "andromeda".to_string()];
        assert_eq!(supported_chains, expected_suuported_chains);

        let query_msg = QueryMsg::ChannelID {
            chain: "juno".to_string(),
        };
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let channel_id: String = from_binary(&res).unwrap();
        let expected_channel_id = "channel-1".to_string();
        assert_eq!(channel_id, expected_channel_id)
    }

    #[test]
    fn test_save_channel_unauthorized() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            kernel_address: Some("kernel_address".to_string()),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let msg = ExecuteMsg::SaveChannel {
            channel: "channel-1".to_owned(),
            chain: "juno".to_owned(),
        };
        let info = mock_info("not_creator", &[]);
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_try_wasm_msg_string() {
        let mut deps = mock_dependencies_custom(&[]);

        let message = to_binary(&"string".to_string()).unwrap();
        let target = "target".to_string();
        let res = try_wasm_msg(deps.as_mut(), target, message.clone()).unwrap();

        let expected_res = WasmMsg::Execute {
            contract_addr: "target".to_string(),
            msg: message,
            funds: vec![],
        };
        assert_eq!(res, expected_res)
    }
}
