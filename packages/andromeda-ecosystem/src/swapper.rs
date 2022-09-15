use common::{
    ado_base::{recipient::Recipient, AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw20::Cw20ReceiveMsg;
use cw_asset::AssetInfo;

#[cw_serde]
pub enum SwapperMsg {
    Swap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

/// Helper enum for calling contracts that implement the Swapper interface.
#[cw_serde]
pub enum SwapperImplExecuteMsg {
    Swapper(SwapperMsg),
}

/// Helper enum for calling contracts that implement the Swapper interface.
#[cw_serde]
pub enum SwapperImplCw20HookMsg {
    Swapper(SwapperCw20HookMsg),
}

#[cw_serde]
pub enum SwapperCw20HookMsg {
    Swap { ask_asset_info: AssetInfo },
}

/// Instantiate Message for Swapper contract.
#[cw_serde]
pub struct InstantiateMsg {
    pub swapper_impl: SwapperImpl,
    pub primitive_contract: String,
}

#[cw_serde]
pub enum SwapperImpl {
    /// Specifies the instantiation specification for the swapper impl.
    New(InstantiateInfo),
    /// Specifies the swapper impl by reference to an existing contract.
    Reference(AndrAddress),
}

#[cw_serde]
pub struct InstantiateInfo {
    /// The instantiate message encoded in base64.
    pub msg: Binary,
    /// The ADO type. Used to retrieve the code id.
    pub ado_type: String,
}

/// Execute Message for Swapper contract.
#[cw_serde]
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
#[cw_serde]
pub enum Cw20HookMsg {
    Swap {
        ask_asset_info: AssetInfo,
        recipient: Option<Recipient>,
    },
}

/// Query Message for Swapper contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
    #[returns(AndrAddress)]
    SwapperImpl {},
}

#[cw_serde]
pub struct MigrateMsg {}
