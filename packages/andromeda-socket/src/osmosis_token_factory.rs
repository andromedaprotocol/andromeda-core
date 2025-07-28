use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_std::error::ContractError;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Env, QuerierWrapper, QueryRequest, Uint128, WasmQuery};
use cw20::{Cw20QueryMsg, Cw20ReceiveMsg, TokenInfoResponse};
use osmosis_std::types::osmosis::gamm::poolmodels::stableswap::v1beta1::PoolParams as StablePoolParams;
use osmosis_std::types::osmosis::gamm::v1beta1::{PoolAsset, PoolParams};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::QueryDenomAuthorityMetadataResponse;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {}

#[andr_exec]
#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    CreateDenom { subdenom: String },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Mint {
        recipient: Option<AndrAddr>,
        subdenom: String,
        amount: Uint128,
    },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Burn {},

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Receive { msg: Cw20ReceiveMsg },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Unlock { recipient: Option<Recipient> },
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Lock the received CW20 tokens and mint factory tokens
    Lock { recipient: Option<AndrAddr> },
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

    #[returns(LockedResponse)]
    Locked { cw20_addr: Addr },

    #[returns(FactoryDenomResponse)]
    FactoryDenom { cw20_addr: Addr },

    #[returns(AllLockedResponse)]
    AllLocked {},
}

#[cw_serde]
pub struct LockedResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct FactoryDenomResponse {
    pub denom: Option<String>,
}

#[cw_serde]
pub struct LockedInfo {
    pub cw20_addr: Addr,
    pub amount: Uint128,
}

#[cw_serde]
pub struct AllLockedResponse {
    pub locked: Vec<LockedInfo>,
}

/// The structure of the newly created denom is: “factory/{osmosis_socket_addr}/{subdenom}}”
pub fn get_factory_denom(env: &Env, subdenom: &str) -> String {
    format!("factory/{}/{}", env.contract.address, subdenom)
}

/// Checks if an address is a cw20 contract by querying the TokenInfo which is guaranteed to error if the address is a wallet
pub fn is_cw20_contract(querier: &QuerierWrapper, address: &str) -> Result<bool, ContractError> {
    Ok(querier
        .query::<TokenInfoResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: address.to_string(),
            msg: to_json_binary(&Cw20QueryMsg::TokenInfo {})?,
        }))
        .is_ok())
}
