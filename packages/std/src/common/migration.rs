use cosmwasm_std::{ensure, Deps};
use cw2::get_contract_version;
use semver::Version;

use crate::error::{from_semver, ContractError};
/// Ensure compatibility when migrating from the previous version.
///
/// min_version specifies the oldest version that is still compatible.
/// If the original version is older than min_version, the migration should fail.
pub fn ensure_compatibility(deps: &Deps, min_version: &str) -> Result<(), ContractError> {
    let prev = get_contract_version(deps.storage)?;
    let prev: Version = prev.version.parse().map_err(from_semver)?;
    let min_version: Version = min_version.parse().unwrap();

    ensure!(
        prev >= min_version,
        ContractError::InvalidMigration {
            prev: prev.to_string()
        }
    );
    Ok(())
}

#[test]
fn test_ensure_compatibility() {
    let mut deps = crate::testing::mock_querier::mock_dependencies_custom(&[]);
    cw2::set_contract_version(&mut deps.storage, "crowdfund", "1.0.0").unwrap();
    let res = ensure_compatibility(&deps.as_ref(), "1.1.0").unwrap_err();
    assert_eq!(
        res,
        ContractError::InvalidMigration {
            prev: "1.0.0".to_string()
        }
    )
}
