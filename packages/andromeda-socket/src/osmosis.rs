use andromeda_std::{
    amp::{AndrAddr, Recipient},
    andr_exec, andr_instantiate,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute;
// use swaprouter::Slippage as OsmosisSlippage;

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
        route: Option<Vec<SwapRoute>>,
    },

    /// Update swap router
    UpdateSwapRouter { swap_router: AndrAddr },
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
pub enum Slippage {
    Twap {
        window_seconds: Option<u64>,
        slippage_percentage: Decimal,
    },
    MinOutputAmount(Uint128),
}

// impl From<Slippage> for OsmosisSlippage {
//     fn from(val: Slippage) -> Self {
//         match val {
//             Slippage::Twap {
//                 window_seconds,
//                 slippage_percentage,
//             } => OsmosisSlippage::Twap {
//                 window_seconds,
//                 slippage_percentage,
//             },
//             Slippage::MinOutputAmount(min_output) => OsmosisSlippage::MinOutputAmount(min_output),
//         }
//     }
// }

#[cw_serde]
pub struct SwapRoute {
    pub pool_id: u64,
    pub token_out_denom: String,
}

impl From<SwapRoute> for SwapAmountInRoute {
    fn from(val: SwapRoute) -> Self {
        SwapAmountInRoute {
            pool_id: val.pool_id,
            token_out_denom: val.token_out_denom,
        }
    }
}

#[cw_serde]
pub struct GetRouteResponse {
    pub pool_route: Vec<SwapRoute>,
}
