use andromeda_std::amp::AndrAddr;
use andromeda_std::os::adodb::ADOVersion;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

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
        admin: Option<AndrAddr>,
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

#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::QueryFns))]
#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
