use astroport::asset::AssetInfo as AstroportAssetInfo;
use common::ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery};
// To be used in the swapper contract.
pub use astroport::querier::{query_balance, query_token_balance};
use cosmwasm_std::{Addr, Binary};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstantiateType {
    New(Binary),
    Address(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperMsg {
    Swap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

/// Helper enum for calling contracts that implement the Swapper interface.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperImplExecuteMsg {
    Swapper(SwapperMsg),
}

/// Helper enum for calling contracts that implement the Swapper interface.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperImplCw20HookMsg {
    Swapper(SwapperCw20HookMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperCw20HookMsg {
    Swap { ask_asset_info: AssetInfo },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}

impl From<AssetInfo> for AstroportAssetInfo {
    fn from(asset_info: AssetInfo) -> AstroportAssetInfo {
        match asset_info {
            AssetInfo::Token { contract_addr } => AstroportAssetInfo::Token { contract_addr },
            AssetInfo::NativeToken { denom } => AstroportAssetInfo::NativeToken { denom },
        }
    }
}

/// Instantiate Message for Swapper contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub swapper_impl: SwapperImpl,
    pub primitive_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapperImpl {
    pub name: String,
    pub instantiate_type: InstantiateType,
}

/// Execute Message for Swapper contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    Receive(Cw20ReceiveMsg),
    Swap {
        ask_asset_info: AssetInfo,
        recipient: Option<Recipient>,
    },
    /// INTERNAL MESSAGE. Sends swapped funds to the recipient.
    Send {
        ask_asset_info: AssetInfo,
        recipient: Recipient,
    },
}

/// Cw20 Hook Message for Swapper contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Swap {
        ask_asset_info: AssetInfo,
        recipient: Option<Recipient>,
    },
}

/// Query Message for Swapper contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
