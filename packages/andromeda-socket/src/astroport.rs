use andromeda_std::{
    amp::{AndrAddr, Recipient},
    andr_exec, andr_instantiate,
    common::denom::Asset,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
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
