use andromeda_std::amp::AndrAddr;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmosisCoin;
use osmosis_std::types::osmosis::gamm::poolmodels::stableswap::v1beta1::PoolParams as StablePoolParams;
use osmosis_std::types::osmosis::gamm::v1beta1::{PoolAsset, PoolParams};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::QueryDenomAuthorityMetadataResponse;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub authorized_address: AndrAddr,
}

#[andr_exec]
#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    CreateDenom {
        subdenom: String,
        amount: Uint128,
        recipient: Option<AndrAddr>,
    },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Mint {
        coin: OsmosisCoin,
        recipient: Option<AndrAddr>,
    },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Burn { coin: OsmosisCoin },
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
    TransferOwnership { new_owner: String },
}
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::QueryFns))]
#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(QueryDenomAuthorityMetadataResponse)]
    TokenAuthority { denom: String },
}
