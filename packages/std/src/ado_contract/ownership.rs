use crate::common::expiration::Expiry;
use crate::common::MillisecondsExpiration;
use crate::error::ContractError;
use crate::{
    ado_base::ownership::{ContractPotentialOwnerResponse, OwnershipMessage},
    ado_contract::ADOContract,
};
use cosmwasm_std::{attr, ensure, Addr, DepsMut, Env, MessageInfo, Response, Storage};
use cw_storage_plus::Item;

const POTENTIAL_OWNER: Item<Addr> = Item::new("andr_potential_owner");
const POTENTIAL_OWNER_EXPIRATION: Item<MillisecondsExpiration> =
    Item::new("andr_potential_owner_expiration");

impl<'a> ADOContract<'a> {
    pub fn execute_ownership(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: OwnershipMessage,
    ) -> Result<Response, ContractError> {
        match msg {
            OwnershipMessage::UpdateOwner {
                new_owner,
                expiration,
            } => self.update_owner(deps, env, info, new_owner, expiration),
            OwnershipMessage::RevokeOwnershipOffer => self.revoke_ownership_offer(deps, info),
            OwnershipMessage::AcceptOwnership => self.accept_ownership(deps, env, info),
            OwnershipMessage::Disown => self.disown(deps, info),
        }
    }

    /// Updates the current contract owner. **Only executable by the current contract owner.**
    pub fn update_owner(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        new_owner: Addr,
        expiration: Option<Expiry>,
    ) -> Result<Response, ContractError> {
        ensure!(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        ensure!(
            !self.is_contract_owner(deps.storage, new_owner.as_str())?,
            ContractError::Unauthorized {}
        );
        let new_owner_addr = deps.api.addr_validate(new_owner.as_ref())?;
        POTENTIAL_OWNER.save(deps.storage, &new_owner_addr)?;

        if let Some(exp) = expiration {
            POTENTIAL_OWNER_EXPIRATION.save(deps.storage, &exp.get_time(&env.block))?;
        } else {
            // In case an offer is already pending
            POTENTIAL_OWNER_EXPIRATION.remove(deps.storage);
        }

        Ok(Response::new().add_attributes(vec![
            attr("action", "update_owner"),
            attr("value", new_owner),
        ]))
    }

    /// Revokes the ownership offer. **Only executable by the current contract owner.**
    pub fn revoke_ownership_offer(
        &self,
        deps: DepsMut,
        info: MessageInfo,
    ) -> Result<Response, ContractError> {
        ensure!(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        POTENTIAL_OWNER.remove(deps.storage);
        POTENTIAL_OWNER_EXPIRATION.remove(deps.storage);
        Ok(Response::new().add_attributes(vec![attr("action", "revoke_ownership_offer")]))
    }

    /// Accepts the ownership of the contract. **Only executable by the new contract owner.**
    pub fn accept_ownership(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
    ) -> Result<Response, ContractError> {
        let new_owner_addr = POTENTIAL_OWNER.load(deps.storage)?;
        ensure!(
            info.sender == new_owner_addr,
            ContractError::Unauthorized {}
        );
        let expiration = POTENTIAL_OWNER_EXPIRATION.may_load(deps.storage)?;
        if let Some(exp) = expiration {
            ensure!(!exp.is_expired(&env.block), ContractError::Unauthorized {});
        }

        self.owner.save(deps.storage, &new_owner_addr)?;
        POTENTIAL_OWNER.remove(deps.storage);
        POTENTIAL_OWNER_EXPIRATION.remove(deps.storage);
        Ok(Response::new().add_attributes(vec![
            attr("action", "accept_ownership"),
            attr("value", new_owner_addr.to_string()),
        ]))
    }

    /// Disowns the contract. **Only executable by the current contract owner.**
    pub fn disown(&self, deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
        ensure!(
            self.is_contract_owner(deps.storage, info.sender.as_str())?,
            ContractError::Unauthorized {}
        );
        self.owner.save(deps.storage, &Addr::unchecked("null"))?;
        Ok(Response::new().add_attributes(vec![attr("action", "disown")]))
    }

    /// Helper function to query if a given address is the current contract owner.
    ///
    /// Returns a boolean value indicating if the given address is the contract owner.
    pub fn owner(&self, storage: &dyn Storage) -> Result<Addr, ContractError> {
        let owner = self.owner.load(storage)?;
        Ok(owner)
    }

    /// Helper function to query if a given address is the current contract owner.
    ///
    /// Returns a boolean value indicating if the given address is the contract owner.
    pub fn is_contract_owner(
        &self,
        storage: &dyn Storage,
        addr: &str,
    ) -> Result<bool, ContractError> {
        let owner = self.owner.load(storage)?;
        Ok(addr == owner)
    }

    /// Helper function to query if a given address is the current contract owner or operator.
    ///
    /// Returns a boolean value indicating if the given address is the contract owner or operator.
    pub fn is_owner_or_operator(
        &self,
        storage: &dyn Storage,
        addr: &str,
    ) -> Result<bool, ContractError> {
        self.is_contract_owner(storage, addr)
    }

    pub fn ownership_request(
        &self,
        storage: &dyn Storage,
    ) -> Result<ContractPotentialOwnerResponse, ContractError> {
        let potential_owner = POTENTIAL_OWNER.may_load(storage)?;
        let expiration = POTENTIAL_OWNER_EXPIRATION.may_load(storage)?;
        Ok(ContractPotentialOwnerResponse {
            potential_owner,
            expiration,
        })
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, DepsMut,
    };

    use crate::{
        ado_contract::{
            ownership::{POTENTIAL_OWNER, POTENTIAL_OWNER_EXPIRATION},
            ADOContract,
        },
        common::MillisecondsExpiration,
    };

    fn init(deps: DepsMut, owner: impl Into<String>) {
        ADOContract::default()
            .owner
            .save(deps.storage, &Addr::unchecked(owner))
            .unwrap();
    }

    #[test]
    fn test_update_owner() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let contract = ADOContract::default();
        let new_owner = Addr::unchecked("new_owner");
        init(deps.as_mut(), "owner");

        let res = contract.update_owner(
            deps.as_mut(),
            env.clone(),
            mock_info("owner", &[]),
            new_owner.clone(),
            None,
        );
        assert!(res.is_ok());
        let saved_new_owner = POTENTIAL_OWNER.load(deps.as_ref().storage).unwrap();
        assert_eq!(saved_new_owner, new_owner);

        let res = contract.update_owner(
            deps.as_mut(),
            env.clone(),
            mock_info("owner", &[]),
            Addr::unchecked("owner"),
            None,
        );
        assert!(res.is_err());
        let res = contract.update_owner(
            deps.as_mut(),
            env,
            mock_info("new_owner", &[]),
            new_owner,
            None,
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_revoke_ownership_offer() {
        let mut deps = mock_dependencies();
        let contract = ADOContract::default();
        init(deps.as_mut(), "owner");

        let res = contract.revoke_ownership_offer(deps.as_mut(), mock_info("owner", &[]));
        assert!(res.is_ok());
        let saved_new_owner = POTENTIAL_OWNER.may_load(deps.as_ref().storage).unwrap();
        assert!(saved_new_owner.is_none());
    }

    #[test]
    fn test_accept_ownership() {
        let mut deps = mock_dependencies();
        let contract = ADOContract::default();
        let new_owner = Addr::unchecked("new_owner");
        init(deps.as_mut(), "owner");
        POTENTIAL_OWNER
            .save(deps.as_mut().storage, &new_owner)
            .unwrap();

        let res = contract.accept_ownership(deps.as_mut(), mock_env(), mock_info("owner", &[]));
        assert!(res.is_err());
        let res = contract.accept_ownership(deps.as_mut(), mock_env(), mock_info("new_owner", &[]));
        assert!(res.is_ok());
        let saved_owner = contract.owner.load(deps.as_ref().storage).unwrap();
        assert_eq!(saved_owner, new_owner);
        let saved_new_owner = POTENTIAL_OWNER.may_load(deps.as_ref().storage).unwrap();
        assert!(saved_new_owner.is_none());
    }

    #[test]
    fn test_accept_ownership_expired() {
        let mut deps = mock_dependencies();
        let contract = ADOContract::default();
        let new_owner = Addr::unchecked("new_owner");
        init(deps.as_mut(), "owner");
        POTENTIAL_OWNER
            .save(deps.as_mut().storage, &new_owner)
            .unwrap();
        POTENTIAL_OWNER_EXPIRATION
            .save(
                deps.as_mut().storage,
                &MillisecondsExpiration::from_nanos(1),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.time = MillisecondsExpiration::from_nanos(2).into();
        let res = contract.accept_ownership(deps.as_mut(), env, mock_info("new_owner", &[]));
        assert!(res.is_err());
        let saved_owner = contract.owner.load(deps.as_ref().storage).unwrap();
        assert_eq!(saved_owner, Addr::unchecked("owner"));
    }

    #[test]
    fn test_disown() {
        let mut deps = mock_dependencies();
        let contract = ADOContract::default();
        init(deps.as_mut(), "owner");

        let res = contract.disown(deps.as_mut(), mock_info("owner", &[]));
        assert!(res.is_ok());
        let saved_owner = contract.owner.load(deps.as_ref().storage).unwrap();
        assert_eq!(saved_owner, Addr::unchecked("null"));
    }
}
