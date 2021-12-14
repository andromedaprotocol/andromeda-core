use cosmwasm_std::{attr, Deps, DepsMut, MessageInfo, Response, StdError, StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::require;

pub const CONTRACT_OWNER: Item<String> = Item::new("contractowner");

/// Helper function to query if a given address is the current contract owner.
///
/// Returns a boolean value indicating if the given address is the contract owner.
pub fn is_contract_owner(storage: &dyn Storage, addr: String) -> StdResult<bool> {
    let owner = CONTRACT_OWNER.load(storage)?;

    Ok(addr.eq(&owner))
}

/// Updates the current contract owner. **Only executable by the current contract owner.**
pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> StdResult<Response> {
    require(
        is_contract_owner(deps.storage, info.sender.to_string())?,
        StdError::generic_err(
            "Ownership of this contract can only be transferred by the current owner",
        ),
    )?;
    //
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;
    CONTRACT_OWNER.save(deps.storage, &new_owner_addr.to_string())?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_owner"),
        attr("value", new_owner),
    ]))
}

pub fn query_contract_owner(deps: Deps) -> StdResult<ContractOwnerResponse> {
    let owner = CONTRACT_OWNER.load(deps.storage)?;

    Ok(ContractOwnerResponse { owner })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractOwnerResponse {
    pub owner: String,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_info};

    use super::*;

    #[test]
    fn test_execute_update_owner() {
        let mut deps = mock_dependencies(&[]);
        let owner = String::from("owner");
        let new_owner = String::from("newowner");

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &owner.clone())
            .unwrap();

        let unauth_info = mock_info("anyone", &[]);

        let resp = execute_update_owner(deps.as_mut(), unauth_info.clone(), String::from("anyone"))
            .unwrap_err();
        let expected = StdError::generic_err(
            "Ownership of this contract can only be transferred by the current owner",
        );
        assert_eq!(resp, expected);

        let auth_info = mock_info(owner.as_str(), &[]);

        let resp =
            execute_update_owner(deps.as_mut(), auth_info.clone(), new_owner.clone()).unwrap();
        let expected = Response::new().add_attributes(vec![
            attr("action", "update_owner"),
            attr("value", new_owner.clone()),
        ]);
        assert_eq!(resp, expected);

        let query_resp = query_contract_owner(deps.as_ref()).unwrap();

        assert_eq!(query_resp.owner, new_owner)
    }
}
