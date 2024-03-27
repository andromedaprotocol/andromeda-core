use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, DepsMut, Response};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::{
    ado_contract::ADOContract,
    error::{from_semver, ContractError},
};

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

pub fn migrate(
    deps: DepsMut,
    contract_name: &str,
    contract_version: &str,
) -> Result<Response, ContractError> {
    // New version
    let version: Version = contract_version.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == contract_name,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, contract_name, contract_version)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}
