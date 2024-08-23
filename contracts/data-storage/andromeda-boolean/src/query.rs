use crate::state::{DATA, DATA_OWNER, RESTRICTION};
use andromeda_data_storage::boolean::{BooleanRestriction, GetDataOwnerResponse, GetValueResponse};
use andromeda_std::{ado_contract::ADOContract, amp::AndrAddr, error::ContractError};
use cosmwasm_std::{Addr, Storage};

pub fn has_permission(storage: &dyn Storage, addr: &Addr) -> Result<bool, ContractError> {
    let is_operator = ADOContract::default().is_owner_or_operator(storage, addr.as_str())?;
    let allowed = match RESTRICTION.load(storage)? {
        BooleanRestriction::Private => is_operator,
        BooleanRestriction::Public => true,
        BooleanRestriction::Restricted => match DATA_OWNER.load(storage).ok() {
            Some(owner) => addr == owner,
            None => true,
        },
    };
    Ok(is_operator || allowed)
}

pub fn get_value(storage: &dyn Storage) -> Result<GetValueResponse, ContractError> {
    let value = DATA.load(storage)?.into();
    Ok(GetValueResponse { value })
}

pub fn get_data_owner(storage: &dyn Storage) -> Result<GetDataOwnerResponse, ContractError> {
    let owner = DATA_OWNER.load(storage)?;
    Ok(GetDataOwnerResponse {
        owner: AndrAddr::from_string(owner),
    })
}
