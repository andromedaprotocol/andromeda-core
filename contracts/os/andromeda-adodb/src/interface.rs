use andromeda_std::{
    ado_base::MigrateMsg,
    contract_interface,
    deploy::ADOMetadata,
    os::adodb::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

pub const CONTRACT_ID: &str = "adodb";

contract_interface!(ADODBContract, CONTRACT_ID, "andromeda_adodb.wasm");

/// Macro to register a contract with the ADODB
///
/// # Arguments
/// * `$env` - The test environment (e.g., juno.aos)
/// * `$contract` - The contract instance
/// * `$ado_type` - The ADO type as a string
/// * `$version` - The version string (defaults to "1.0.0")
#[macro_export]
macro_rules! register_contract {
    ($env:expr, $contract:expr, $ado_type:expr) => {
        register_contract!($env, $contract, $ado_type, "1.0.0")
    };
    ($env:expr, $contract:expr, $ado_type:expr, $version:expr) => {
        $env.adodb
            .execute(
                &os::adodb::ExecuteMsg::Publish {
                    code_id: $contract.code_id().unwrap(),
                    ado_type: $ado_type.to_string(),
                    action_fees: None,
                    version: $version.to_string(),
                    publisher: None,
                },
                None,
            )
            .unwrap()
    };
}
