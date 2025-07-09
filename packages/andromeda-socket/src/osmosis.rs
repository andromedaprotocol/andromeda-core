use andromeda_std::{
    amp::{AndrAddr, Recipient},
    andr_exec, andr_instantiate, andr_query,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Uint128};
use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmosisCoin;
use osmosis_std::types::osmosis::gamm::v1beta1::{PoolAsset, PoolParams};
use osmosis_std::types::osmosis::gamm::{
    poolmodels::stableswap::v1beta1::PoolParams as StablePoolParams, v1beta1::MsgExitPool,
};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::QueryDenomAuthorityMetadataResponse;

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
    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    CreatePool { pool_type: Pool },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    WithdrawPool { withdraw_msg: MsgExitPool },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    CreateDenom { subdenom: String, amount: Uint128 },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Mint { coin: OsmosisCoin },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Burn { coin: OsmosisCoin },

    /// Update swap router
    #[attrs(restricted)]
    UpdateSwapRouter { swap_router: AndrAddr },
}

#[cw_serde]
pub enum Pool {
    Balancer {
        pool_params: Option<PoolParams>,
        pool_assets: Vec<PoolAsset>,
    },
    Stable {
        pool_params: Option<StablePoolParams>,
        scaling_factors: Vec<u64>,
    },
    Concentrated {
        tick_spacing: u64,
        spread_factor: String,
    },
    CosmWasm {
        code_id: u64,
        instantiate_msg: Vec<u8>,
    },
}

#[cw_serde]
pub enum OsmosisExecuteMsg {
    TransferOwnership {
        new_owner: String,
    },
    SetRoute {
        input_denom: String,
        output_denom: String,
        pool_route: Vec<SwapAmountInRoute>,
    },
    Swap {
        input_coin: Coin,
        output_denom: String,
        slippage: Slippage,
        route: Option<Vec<SwapAmountInRoute>>,
    },
}
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::QueryFns))]
#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetRouteResponse)]
    GetRoute {
        from_denom: String,
        to_denom: String,
    },
    #[returns(QueryDenomAuthorityMetadataResponse)]
    TokenAuthority { denom: String },
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
    pub pool_id: String,
    pub token_out_denom: String,
}

#[cw_serde]
pub struct GetRouteResponse {
    pub pool_route: Vec<SwapAmountInRoute>,
}
