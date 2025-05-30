use andromeda_std::{
    amp::{AndrAddr, Recipient},
    andr_exec, andr_instantiate,
    common::denom::Asset,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub swap_router: Option<AndrAddr>,
}

#[andr_exec]
#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    /// Swap cw20 asset into another asset using astroport
    Receive(Cw20ReceiveMsg),
    /// Swap native token into another asset using astroport
    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    SwapAndForward {
        /// The asset swap to be swapped to
        to_asset: Asset,
        /// The recipient where the swapped token is supposed to be sent
        recipient: Option<Recipient>,
        /// The max spread. Equals to slippage tolerance / 100
        max_spread: Option<Decimal>,
        /// The minimum amount of tokens to receive from swap operation
        minimum_receive: Option<Uint128>,
        /// The swap operations that is supposed to be taken
        operations: Option<Vec<SwapOperation>>,
    },
    /// Update swap router
    #[attrs(restricted)]
    UpdateSwapRouter { swap_router: AndrAddr },
}

#[cw_serde]
pub enum Cw20HookMsg {
    SwapAndForward {
        /// The asset swap to be swapped to
        to_asset: Asset,
        /// The recipient where the swapped token is supposed to be sent
        recipient: Option<Recipient>,
        /// The max spread. Equals to slippage tolerance / 100
        max_spread: Option<Decimal>,
        /// The minimum amount of tokens to receive from swap operation
        minimum_receive: Option<Uint128>,
        /// The swap operations that is supposed to be taken
        operations: Option<Vec<SwapOperation>>,
    },
}
#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::QueryFns))]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(SimulateSwapOperationResponse)]
    SimulateSwapOperation {
        /// The amount of tokens to swap
        offer_amount: Uint128,
        /// The swap operation to perform
        operations: Vec<SwapOperation>,
    },
}

#[cw_serde]
pub struct SwapOperation {
    /// The asset being swapped
    pub offer_asset_info: Asset,
    /// The asset swap to be swapped to
    pub ask_asset_info: Asset,
}

#[cw_serde]
pub struct SimulateSwapOperationResponse {
    /// The expected amount of tokens being received from swap operation
    pub amount: Uint128,
}

// Imports directly from astroport
/// This enum describes available Token types.
#[cw_serde]
#[derive(Hash, Eq)]
pub enum AssetInfo {
    /// Non-native Token
    Token { contract_addr: Addr },
    /// Native token
    NativeToken { denom: String },
}

pub const MAX_SWAP_OPERATIONS: usize = 50;

/// This structure holds the parameters used for creating a contract.
#[cw_serde]
pub struct InstantiateMsgAstroport {
    /// The astroport factory contract address
    pub astroport_factory: String,
}

/// This enum describes a swap operation.
#[cw_serde]
pub enum SwapOperationAstroport {
    /// Native swap
    NativeSwap {
        /// The name (denomination) of the native asset to swap from
        offer_denom: String,
        /// The name (denomination) of the native asset to swap to
        ask_denom: String,
    },
    /// ASTRO swap
    AstroSwap {
        /// Information about the asset being swapped
        offer_asset_info: AssetInfo,
        /// Information about the asset we swap to
        ask_asset_info: AssetInfo,
    },
}

impl SwapOperationAstroport {
    pub fn get_target_asset_info(&self) -> AssetInfo {
        match self {
            SwapOperationAstroport::NativeSwap { ask_denom, .. } => AssetInfo::NativeToken {
                denom: ask_denom.clone(),
            },
            SwapOperationAstroport::AstroSwap { ask_asset_info, .. } => ask_asset_info.clone(),
        }
    }
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsgAstroport {
    /// Receive receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template
    Receive(Cw20ReceiveMsg),
    /// ExecuteSwapOperations processes multiple swaps while mentioning the minimum amount of tokens to receive for the last swap operation
    ExecuteSwapOperations {
        operations: Vec<SwapOperationAstroport>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        max_spread: Option<Decimal>,
    },

    /// Internal use
    /// ExecuteSwapOperation executes a single swap operation
    ExecuteSwapOperation {
        operation: SwapOperationAstroport,
        to: Option<String>,
        max_spread: Option<Decimal>,
        single: bool,
    },
}

#[cw_serde]
pub struct SwapResponseData {
    pub return_amount: Uint128,
}

#[cw_serde]
pub enum Cw20HookMsgAstroport {
    ExecuteSwapOperations {
        /// A vector of swap operations
        operations: Vec<SwapOperationAstroport>,
        /// The minimum amount of tokens to get from a swap
        minimum_receive: Option<Uint128>,
        /// The recipient
        to: Option<String>,
        /// Max spread
        max_spread: Option<Decimal>,
    },
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsgAstroport {
    /// Config returns configuration parameters for the contract using a custom [`ConfigResponse`] structure
    #[returns(ConfigResponse)]
    Config {},
    /// SimulateSwapOperations simulates multi-hop swap operations
    #[returns(SimulateSwapOperationsResponse)]
    SimulateSwapOperations {
        /// The amount of tokens to swap
        offer_amount: Uint128,
        /// The swap operations to perform, each swap involving a specific pool
        operations: Vec<SwapOperationAstroport>,
    },
    #[returns(Uint128)]
    ReverseSimulateSwapOperations {
        /// The amount of tokens one is willing to receive
        ask_amount: Uint128,
        /// The swap operations to perform, each swap involving a specific pool
        operations: Vec<SwapOperationAstroport>,
    },
}

/// This structure describes a custom struct to return a query response containing the base contract configuration.
#[cw_serde]
pub struct ConfigResponse {
    /// The Astroport factory contract address
    pub astroport_factory: String,
}

/// This structure describes a custom struct to return a query response containing the end amount of a swap simulation
#[cw_serde]
pub struct SimulateSwapOperationsResponse {
    /// The amount of tokens received in a swap simulation
    pub amount: Uint128,
}
