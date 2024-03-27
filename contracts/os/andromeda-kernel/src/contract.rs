use andromeda_std::ado_base::InstantiateMsg as BaseInstantiateMsg;
use andromeda_std::ado_contract::ADOContract;

use andromeda_std::common::context::ExecuteContext;
use andromeda_std::common::encode_binary;
use andromeda_std::common::migrate::{migrate as do_migrate, MigrateMsg};
use andromeda_std::error::ContractError;

use andromeda_std::os::kernel::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};
use cw2::set_contract_version;

use crate::ibc::{IBCLifecycleComplete, SudoMsg};
use crate::reply::{on_reply_create_ado, on_reply_ibc_hooks_packet_send, ReplyId};
use crate::state::CURR_CHAIN;
use crate::{execute, query, sudo};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-kernel";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CURR_CHAIN.save(deps.storage, &msg.chain_name)?;

    ADOContract::default().instantiate(
        deps.storage,
        env.clone(),
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "kernel".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: env.contract.address.to_string(),
            owner: msg.owner,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "{}:{}",
            msg.id,
            msg.result.unwrap_err()
        ))));
    }

    match ReplyId::from_repr(msg.id) {
        Some(ReplyId::CreateADO) => on_reply_create_ado(deps, env, msg),
        Some(ReplyId::IBCHooksPacketSend) => on_reply_ibc_hooks_packet_send(deps, msg),
        _ => Ok(Response::default()),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut execute_env = ExecuteContext {
        deps,
        env,
        info,
        amp_ctx: None,
    };

    match msg {
        ExecuteMsg::AMPReceive(packet) => execute::amp_receive(
            &mut execute_env.deps,
            execute_env.info,
            execute_env.env,
            packet,
        ),
        ExecuteMsg::Send { message } => execute::send(execute_env, message),
        ExecuteMsg::UpsertKeyAddress { key, value } => {
            execute::upsert_key_address(execute_env, key, value)
        }
        ExecuteMsg::Create {
            ado_type,
            msg,
            owner,
            chain,
        } => execute::create(execute_env, ado_type, msg, owner, chain),
        ExecuteMsg::AssignChannels {
            ics20_channel_id,
            direct_channel_id,
            chain,
            kernel_address,
        } => execute::assign_channels(
            execute_env,
            ics20_channel_id,
            direct_channel_id,
            chain,
            kernel_address,
        ),
        ExecuteMsg::Recover {} => execute::recover(execute_env),
        ExecuteMsg::Internal(msg) => execute::internal(execute_env, msg),
    }
}

#[cfg_attr(not(feature = "imported"), entry_point)]
pub fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::IBCLifecycleComplete(IBCLifecycleComplete::IBCAck {
            channel,
            sequence,
            ack,
            success,
        }) => sudo::ibc_lifecycle::receive_ack(deps, channel, sequence, ack, success),
        SudoMsg::IBCLifecycleComplete(IBCLifecycleComplete::IBCTimeout { channel, sequence }) => {
            sudo::ibc_lifecycle::receive_timeout(deps, channel, sequence)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    do_migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::KeyAddress { key } => encode_binary(&query::key_address(deps, key)?),
        QueryMsg::VerifyAddress { address } => {
            encode_binary(&query::verify_address(deps, address)?)
        }
        QueryMsg::ChannelInfo { chain } => encode_binary(&query::channel_info(deps, chain)?),
        QueryMsg::Recoveries { addr } => encode_binary(&query::recoveries(deps, addr)?),
    }
}
