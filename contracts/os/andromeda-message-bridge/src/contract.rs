use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::AndrAddr;
use andromeda_std::common::encode_binary;
use andromeda_std::error::from_semver;
use andromeda_std::ibc::message_bridge::{ExecuteMsg, IbcExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::os::kernel::MigrateMsg;
use andromeda_std::{ado_base::InstantiateMsg as BaseInstantiateMsg, error::ContractError};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, ensure, to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response,
    WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::one_coin;
use semver::Version;

use crate::ibc::{generate_transfer_message, receive_ack, IBCLifecycleComplete, SudoMsg};
use crate::state::{
    read_chains, read_channel, save_channel, update_channel, CHAIN_KERNEL_ADDRESSES,
};

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
            kernel_address: msg.kernel_address,
            owner: None,
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
    let _contract = ADOContract::default();
    match msg {
        ExecuteMsg::SendMessage {
            chain,
            recipient,
            message,
        } => execute_send_message(deps, env, info, chain, recipient, message),
        ExecuteMsg::SaveChannel {
            channel,
            chain,
            kernel_address,
        } => execute_save_channel(deps, info, channel, chain, kernel_address),
        ExecuteMsg::UpdateChannel {
            channel,
            chain,
            kernel_address,
        } => execute_update_channel(deps, info, channel, chain, kernel_address),
        _ => Err(ContractError::Unauthorized {}),
    }
}

pub fn execute_send_message(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    chain: String,
    recipient: AndrAddr,
    message: Binary,
) -> Result<Response, ContractError> {
    let channel = read_channel(deps.storage, chain.clone())?;

    // IbcTransfer supports only one coin at a time
    one_coin(&info)?;

    if info.funds.is_empty() {
        Ok(Response::new()
            .add_attribute("method", "execute_send_message")
            .add_attribute("channel", channel.clone())
            .add_attribute("chain", chain)
            // outbound IBC message, where packet is then received on other chain
            .add_message(IbcMsg::SendPacket {
                channel_id: channel,
                data: to_binary(&IbcExecuteMsg::SendMessage {
                    recipient: AndrAddr::from_string(recipient.get_raw_path()),
                    message,
                })?,
                timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(300)),
            }))
    } else {
        // let port = env.contract.address.to_string();
        let funds = &info.funds[0];
        let kernel_address = CHAIN_KERNEL_ADDRESSES.load(deps.storage, chain.clone())?;

        let msg = generate_transfer_message(
            recipient,
            message,
            funds.clone(),
            channel.clone(),
            env.contract.address.to_string(),
            kernel_address,
            env.block.time,
        )?;
        Ok(Response::new()
            .add_attribute("method", "execute_send_message")
            .add_attribute("channel", channel)
            .add_attribute("chain", chain)
            .add_message(msg))
    }
}

pub fn execute_update_channel(
    deps: DepsMut,
    info: MessageInfo,
    channel: String,
    chain: String,
    kernel_address: Option<String>,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    update_channel(deps.storage, chain.clone(), channel.clone())?;
    if let Some(kernel_address) = kernel_address {
        CHAIN_KERNEL_ADDRESSES.save(deps.storage, chain.clone(), &kernel_address)?;
    }

    Ok(Response::default().add_attributes(vec![
        attr("action", "execute_save_channel"),
        attr("chain", chain),
        attr("channel", channel),
    ]))
}

pub fn execute_save_channel(
    deps: DepsMut,
    info: MessageInfo,
    channel: String,
    chain: String,
    kernel_address: String,
) -> Result<Response, ContractError> {
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    save_channel(deps.storage, chain.clone(), channel.clone())?;
    CHAIN_KERNEL_ADDRESSES.save(deps.storage, chain.clone(), &kernel_address)?;

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
    // The message is supposed to be an AMPPkt in binary form
    // Further testing will guide how to handle funds
    Ok(WasmMsg::Execute {
        contract_addr: kernel_address,
        msg: message,
        funds: vec![],
    })
}

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

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

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::IBCLifecycleComplete(IBCLifecycleComplete::IBCAck {
            channel,
            sequence,
            ack,
            success,
        }) => receive_ack(deps, channel, sequence, ack, success),
        SudoMsg::IBCLifecycleComplete(IBCLifecycleComplete::IBCTimeout {
            channel: _,
            sequence: _,
        }) => Ok(Response::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CHAIN_CHANNELS;
    use andromeda_std::testing::mock_querier::{mock_dependencies_custom, MOCK_KERNEL_CONTRACT};
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            kernel_address: "kernel_address".to_string(),
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
            kernel_address: "kernel_address".to_string(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
        let msg = ExecuteMsg::SaveChannel {
            channel: "channel-1".to_owned(),
            chain: "juno".to_owned(),
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::SaveChannel {
            channel: "channel-2".to_owned(),
            chain: "andromeda".to_owned(),
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
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
    fn test_update_channel() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            kernel_address: "kernel_address".to_string(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
        let msg = ExecuteMsg::SaveChannel {
            channel: "channel-1".to_owned(),
            chain: "juno".to_owned(),
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::SaveChannel {
            channel: "channel-2".to_owned(),
            chain: "andromeda".to_owned(),
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let channel_id: String = from_binary(&res).unwrap();
        let expected_channel_id = "channel-1".to_string();
        assert_eq!(channel_id, expected_channel_id);

        let update_msg = ExecuteMsg::UpdateChannel {
            channel: "channel-2".to_string(),
            chain: "juno".to_string(),
            kernel_address: None,
        };
        let _res = execute(deps.as_mut(), env.clone(), info, update_msg).unwrap();

        let query_msg = QueryMsg::ChannelID {
            chain: "juno".to_string(),
        };
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let channel_id: String = from_binary(&res).unwrap();
        let expected_channel_id = "channel-2".to_string();
        assert_eq!(channel_id, expected_channel_id);
    }

    #[test]
    fn test_save_channel_unauthorized() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            kernel_address: "kernel_address".to_string(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let msg = ExecuteMsg::SaveChannel {
            channel: "channel-1".to_owned(),
            chain: "juno".to_owned(),
            kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
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
