use andromeda_std::{
    amp::ADO_DB_KEY,
    error::ContractError,
    os::{aos_querier::AOSQuerier, kernel::ChannelInfoResponse},
};
use cosmwasm_std::{Addr, Coin, Deps};

use crate::state::{CHAIN_TO_CHANNEL, IBC_FUND_RECOVERY, KERNEL_ADDRESSES};

pub fn key_address(deps: Deps, key: String) -> Result<Addr, ContractError> {
    Ok(KERNEL_ADDRESSES.load(deps.storage, &key)?)
}

pub fn verify_address(deps: Deps, address: String) -> Result<bool, ContractError> {
    let db_address = KERNEL_ADDRESSES.load(deps.storage, ADO_DB_KEY)?;
    let contract_info_res = deps.querier.query_wasm_contract_info(address);
    if let Ok(contract_info) = contract_info_res {
        let ado_type =
            AOSQuerier::ado_type_getter(&deps.querier, &db_address, contract_info.code_id)
                .ok()
                .ok_or(ContractError::InvalidAddress {})?;
        Ok(ado_type.is_some())
    } else {
        Ok(false)
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

pub fn recoveries(deps: Deps, addr: Addr) -> Result<Vec<Coin>, ContractError> {
    Ok(IBC_FUND_RECOVERY
        .may_load(deps.storage, &addr)?
        .unwrap_or_default())
}
