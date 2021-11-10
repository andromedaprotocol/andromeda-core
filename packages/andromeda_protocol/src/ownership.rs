use cosmwasm_std::{
    attr, Addr, Api, Deps, DepsMut, MessageInfo, Response, StdError, StdResult, Storage,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::require::require;

pub const CONTRACT_OWNER: Item<Addr> = Item::new("contractowner");

pub fn is_contract_owner(storage: &dyn Storage, addr: String) -> StdResult<bool> {
    let owner = CONTRACT_OWNER.load(storage)?;

    Ok(addr.eq(&owner))
}

pub fn save_owner(storage: &mut dyn Storage, api: &dyn Api, addr: String) -> StdResult<()> {
    let new_owner_addr = api.addr_validate(&addr)?;
    CONTRACT_OWNER.save(storage, &new_owner_addr.clone())?;
    Ok(())
}

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
    save_owner(deps.storage, deps.api, new_owner.clone())?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "update_owner"),
        attr("value", new_owner.clone()),
    ]))
}

pub fn query_contract_owner(deps: Deps) -> StdResult<ContractOwnerResponse> {
    let owner = CONTRACT_OWNER.load(deps.storage)?;

    Ok(ContractOwnerResponse {
        owner: owner.to_string(),
    })
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
        let new_owner_addr = Addr::unchecked(owner.clone());
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &new_owner_addr.clone())
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
