use crate::communication::{modules::InstantiateType, Recipient};
use astroport::asset::AssetInfo as AstroportAssetInfo;
// To be used in the swapper contract.
pub use astroport::querier::{query_balance, query_token_balance};
use cosmwasm_std::Addr;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperMsg {
    Swap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// Helper enum for calling contracts that implement the Swapper interface.
pub enum SwapperImplExecuteMsg {
    Swapper(SwapperMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// Helper enum for calling contracts that implement the Swapper interface.
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Instantiate Message for Swapper contract.
pub struct InstantiateMsg {
    pub swapper_impl: InstantiateType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// Execute Message for Swapper contract.
pub enum ExecuteMsg {
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// Cw20 Hook Message for Swapper contract.
pub enum Cw20HookMsg {
    Swap {
        ask_asset_info: AssetInfo,
        recipient: Option<Recipient>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// Query Message for Swapper contract.
pub enum QueryMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
