use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;
use andromeda_std::os::adodb::ADOVersion;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Env, QuerierWrapper, QueryRequest, Uint128, WasmQuery,
};
use cw20::{Cw20QueryMsg, TokenInfoResponse};

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub admins: Vec<String>,
}

#[cw_serde]
pub enum InitParams {
    CodeId(u64),
    AdoVersion(ADOVersion),
}

#[andr_exec]
#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    Instantiate {
        init_params: InitParams,
        message: Binary,
        admin: Option<String>,
        label: Option<String>,
    },

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    Execute {
        contract_addr: AndrAddr,
        message: Binary,
        // Funds will be native
    },

    ModifyAdmins {
        admins: Vec<String>,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Lock the received CW20 tokens and mint factory tokens
    Lock { recipient: Option<AndrAddr> },
}

#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::QueryFns))]
#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LockedResponse)]
    Locked { cw20_addr: Addr },

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
