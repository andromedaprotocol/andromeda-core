use cosmwasm_std::{attr, Addr, Deps, DepsMut, MessageInfo, Response, StdResult, Storage};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use crate::require;
use terraswap::asset::AssetInfo;

pub const WITHDRAWABLE_TOKENS: Map<&str, AssetInfo> = Map::new("withdrawable_tokens");

pub fn add_token(
    storage: &mut dyn Storage,
    name: &str,
    asset_info: &AssetInfo,
) -> Result<(), ContractError> {
    Ok(WITHDRAWABLE_TOKENS.save(storage, name, asset_info)?)
}

pub fn remove_token(storage: &mut dyn Storage, name: &str) -> Result<(), ContractError> {
    Ok(WITHDRAWABLE_TOKENS.remove(storage, name))
}
