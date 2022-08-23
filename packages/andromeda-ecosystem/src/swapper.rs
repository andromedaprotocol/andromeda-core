use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_std::Binary;
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperMsg {
    Swap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

/// Helper enum for calling contracts that implement the Swapper interface.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperImplExecuteMsg {
    Swapper(SwapperMsg),
}

/// Helper enum for calling contracts that implement the Swapper interface.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperImplCw20HookMsg {
    Swapper(SwapperCw20HookMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperCw20HookMsg {
    Swap { ask_asset_info: AssetInfo },
}

/// Instantiate Message for Swapper contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub swapper_impl: SwapperImpl,
    pub primitive_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapperImpl {
    /// Specifies the instantiation specification for the swapper impl.
    New(InstantiateInfo),
    /// Specifies the swapper impl by reference to an existing contract.
    Reference(AndrAddress),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateInfo {
    /// The instantiate message encoded in base64.
    pub msg: Binary,
    /// The ADO type. Used to retrieve the code id.
    pub ado_type: String,
}

/// Execute Message for Swapper contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Swap {
        ask_asset_info: AssetInfo,
        recipient: Option<Recipient>,
    },
}

/// Query Message for Swapper contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    SwapperImpl {},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
