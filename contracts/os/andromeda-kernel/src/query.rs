use andromeda_std::{
    amp::ADO_DB_KEY,
    error::ContractError,
    os::{
        aos_querier::AOSQuerier,
        kernel::{
            ChainNameResponse, ChannelInfoResponse, EnvResponse, PacketInfoAndSequence,
            PendingPacketResponse, VerifyAddressResponse,
        },
    },
};
use cosmwasm_std::{Addr, Coin, Deps, Order};

use crate::state::{
    CHAIN_TO_CHANNEL, CHANNEL_TO_CHAIN, CHANNEL_TO_EXECUTE_MSG, CURR_CHAIN, ENV_VARIABLES,
    IBC_FUND_RECOVERY, KERNEL_ADDRESSES,
};

pub fn key_address(deps: Deps, key: String) -> Result<Addr, ContractError> {
    Ok(KERNEL_ADDRESSES.load(deps.storage, &key)?)
}

pub fn verify_address(deps: Deps, address: String) -> Result<VerifyAddressResponse, ContractError> {
    let db_address = KERNEL_ADDRESSES.load(deps.storage, ADO_DB_KEY)?;
    let contract_info_res = deps.querier.query_wasm_contract_info(address);
    if let Ok(contract_info) = contract_info_res {
        let ado_type =
            AOSQuerier::ado_type_getter(&deps.querier, &db_address, contract_info.code_id)
                .ok()
                .ok_or(ContractError::InvalidAddress {})?;
        Ok(VerifyAddressResponse {
            verify_address: ado_type.is_some(),
        })
    } else {
        Ok(VerifyAddressResponse {
            verify_address: false,
        })
    }
}

pub fn channel_info(
    deps: Deps,
    chain: String,
) -> Result<Option<ChannelInfoResponse>, ContractError> {
    let info = CHAIN_TO_CHANNEL.may_load(deps.storage, &chain)?;
    let resp = if let Some(info) = info {
        Some(ChannelInfoResponse {
            ics20: info.ics20_channel_id,
            direct: info.direct_channel_id,
            kernel_address: info.kernel_address,
            supported_modules: info.supported_modules,
        })
    } else {
        None
    };
    Ok(resp)
}

pub fn chain_name_by_channel(deps: Deps, channel: String) -> Result<Option<String>, ContractError> {
    let info = CHANNEL_TO_CHAIN.may_load(deps.storage, &channel)?;
    Ok(info)
}

pub fn recoveries(deps: Deps, addr: Addr) -> Result<Vec<Coin>, ContractError> {
    Ok(IBC_FUND_RECOVERY
        .may_load(deps.storage, &addr)?
        .unwrap_or_default())
}

pub fn chain_name(deps: Deps) -> Result<ChainNameResponse, ContractError> {
    Ok(ChainNameResponse {
        chain_name: CURR_CHAIN.may_load(deps.storage)?.unwrap_or_default(),
    })
}

pub fn pending_packets(
    deps: Deps,
    channel_id: Option<String>,
) -> Result<PendingPacketResponse, ContractError> {
    let packets: Vec<PacketInfoAndSequence> = if let Some(channel_id) = channel_id {
        CHANNEL_TO_EXECUTE_MSG
            .prefix(channel_id)
            .range(deps.storage, None, None, Order::Ascending)
            .filter_map(|item| item.ok())
            .map(|(sequence, packet)| PacketInfoAndSequence {
                packet_info: packet,
                sequence,
            })
            .collect()
    } else {
        CHANNEL_TO_EXECUTE_MSG
            .range(deps.storage, None, None, Order::Ascending)
            .filter_map(|item| item.ok())
            .map(|((_, sequence), packet)| PacketInfoAndSequence {
                packet_info: packet,
                sequence,
            })
            .collect()
    };
    Ok(PendingPacketResponse { packets })
}

pub fn get_env(deps: Deps, variable: String) -> Result<EnvResponse, ContractError> {
    Ok(EnvResponse {
        value: ENV_VARIABLES.may_load(deps.storage, &variable.to_ascii_uppercase())?,
    })
}
