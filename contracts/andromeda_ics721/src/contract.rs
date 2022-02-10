#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, IbcMsg, IbcQuery, MessageInfo, Order,
    PortIdResponse, QueryRequest, Response, StdResult, WasmQuery,
};

use cw2::{get_contract_version, set_contract_version};

use crate::error::ContractError;
use crate::ibc::Ics721Packet;
use crate::msg::{
    ChannelResponse, ExecuteMsg, InitMsg, ListChannelsResponse, MigrateMsg, PortResponse, QueryMsg,
    TransferMsg,
};
use crate::state::{Config, CHANNEL_INFO, CONFIG};
use andromeda_protocol::token::NftInfoResponseExtension;
use cw0::nonpayable;
use cw721::{Cw721QueryMsg, Cw721ReceiveMsg, NftInfoResponse};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-ics721";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = Config {
        default_timeout: msg.default_timeout,
    };
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let msg: TransferMsg = from_binary(&wrapper.msg)?;

    let token: NftInfoResponse<NftInfoResponseExtension> =
        query_nft_info(deps.as_ref(), info.sender.to_string(), &wrapper.token_id)?;

    let api = deps.api;
    execute_transfer(
        deps,
        env,
        msg,
        wrapper.token_id.clone(),
        info.sender.to_string(),
        token,
        api.addr_validate(&wrapper.sender)?,
    )
}

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    msg: TransferMsg,
    token_id: String,
    token_addr: String,
    token: NftInfoResponse<NftInfoResponseExtension>,
    sender: Addr,
) -> Result<Response, ContractError> {
    if !CHANNEL_INFO.has(deps.storage, &msg.channel) {
        return Err(ContractError::NoSuchChannel { id: msg.channel });
    }

    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => CONFIG.load(deps.storage)?.default_timeout,
    };
    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(timeout_delta);

    // build ics721 packet
    let packet = Ics721Packet::new(
        token_id,
        token_addr,
        token,
        sender.as_ref(),
        &msg.remote_address,
    );

    // prepare message
    let msg = IbcMsg::SendPacket {
        channel_id: msg.channel,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    };

    // Note: we update local state when we get ack - do not count this transfer towards anything until acked
    // similar event messages like ibctransfer module

    // send response
    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer")
        .add_attribute("sender", &packet.sender)
        .add_attribute("receiver", &packet.receiver);

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Port {} => to_binary(&query_port(deps)?),
        QueryMsg::ListChannels {} => to_binary(&query_list(deps)?),
        QueryMsg::Channel { id } => to_binary(&query_channel(deps, id)?),
    }
}

fn query_port(deps: Deps) -> StdResult<PortResponse> {
    let query = IbcQuery::PortId {}.into();
    let PortIdResponse { port_id } = deps.querier.query(&query)?;
    Ok(PortResponse { port_id })
}

fn query_list(deps: Deps) -> StdResult<ListChannelsResponse> {
    let channels: StdResult<Vec<_>> = CHANNEL_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect();
    Ok(ListChannelsResponse {
        channels: channels?,
    })
}

// make public for ibc tests
pub fn query_channel(deps: Deps, id: String) -> StdResult<ChannelResponse> {
    let info = CHANNEL_INFO.load(deps.storage, &id)?;
    // we want (Vec<outstanding>, Vec<total>)

    Ok(ChannelResponse { info })
}
pub fn query_nft_info(
    deps: Deps,
    contract_addr: String,
    token_id: &String,
) -> StdResult<NftInfoResponse<NftInfoResponseExtension>> {
    let token_info: NftInfoResponse<NftInfoResponseExtension> =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr,
            msg: to_binary(&Cw721QueryMsg::NftInfo {
                token_id: token_id.to_string(),
            })?,
        }))?;
    Ok(token_info)
}
