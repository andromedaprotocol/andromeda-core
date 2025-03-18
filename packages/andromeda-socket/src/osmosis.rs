use andromeda_std::{
    amp::{AndrAddr, Recipient},
    andr_exec, andr_instantiate,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Uint128};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub swap_router: Option<AndrAddr>,
}

#[andr_exec]
#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    /// Swap native token into another asset using osmosis
    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    SwapAndForward {
        /// The asset swap to be swapped to
        to_denom: String,
        /// The recipient where the swapped token is supposed to be sent
        recipient: Option<Recipient>,
        /// The slippage
        slippage: Slippage,
        /// The swap operations that is supposed to be taken
        route: Option<Vec<SwapAmountInRoute>>,
    },

    /// Update swap router
    UpdateSwapRouter { swap_router: AndrAddr },
}

#[cw_serde]
pub enum OsmosisExecuteMsg {
    Swap {
        input_coin: Coin,
        output_denom: String,
        slippage: Slippage,
        route: Option<Vec<SwapAmountInRoute>>,
    },
}

#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::QueryFns))]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetRouteResponse)]
    GetRoute {
        from_denom: String,
        to_denom: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum OsmosisQueryMsg {
    #[returns(GetRouteResponse)]
    GetRoute {
        input_denom: String,
        output_denom: String,
    },
}

#[cw_serde]
pub enum Slippage {
    Twap {
        window_seconds: Option<u64>,
        slippage_percentage: Decimal,
    },
    MinOutputAmount(Uint128),
}

#[cw_serde]
pub struct SwapAmountInRoute {
    pub pool_id: u64,
    pub token_out_denom: String,
}

#[cw_serde]
pub struct GetRouteResponse {
    pub pool_route: Vec<SwapAmountInRoute>,
}
