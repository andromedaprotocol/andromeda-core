use crate::state::{DATA, DATA_OWNER, RESTRICTION};
use andromeda_math::point::{GetDataOwnerResponse, PointCoordinate, PointRestriction};
use andromeda_std::{ado_contract::ADOContract, amp::AndrAddr, error::ContractError};
use cosmwasm_std::{Addr, Storage};

pub fn has_permission(storage: &dyn Storage, addr: &Addr) -> Result<bool, ContractError> {
    let is_operator = ADOContract::default().is_owner_or_operator(storage, addr.as_str())?;
    let allowed = match RESTRICTION.load(storage)? {
        PointRestriction::Private => is_operator,
        PointRestriction::Public => true,
        PointRestriction::Restricted => match DATA_OWNER.load(storage).ok() {
            Some(owner) => addr == owner,
            None => true,
        },
    };
    Ok(is_operator || allowed)
}

pub fn get_point(storage: &dyn Storage) -> Result<PointCoordinate, ContractError> {
    let point = DATA.load(storage)?;
    Ok(point)
}

pub fn get_data_owner(storage: &dyn Storage) -> Result<GetDataOwnerResponse, ContractError> {
    let owner = DATA_OWNER.load(storage)?;
    Ok(GetDataOwnerResponse {
        owner: AndrAddr::from_string(owner),
    })
}
