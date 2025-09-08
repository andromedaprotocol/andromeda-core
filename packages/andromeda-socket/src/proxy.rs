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

#[cw_serde]
pub struct Operation {
    pub execution: ExecutionType,
    pub fail_on_error: bool,
}

#[cw_serde]
pub enum ExecutionType {
    Instantiate {
        init_params: InitParams,
        message: Binary,
        admin: Option<AndrAddr>,
        label: Option<String>,
    },
    Execute {
        contract_addr: AndrAddr,
        message: Binary,
    },
    Migrate {
        contract_addr: AndrAddr,
        new_code_id: u64,
        migrate_msg: Binary,
    },
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

    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    BatchExecute {
        operations: Vec<Operation>,
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
