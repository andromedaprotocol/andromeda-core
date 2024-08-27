use crate::state::{DATA, DATA_OWNER};
use andromeda_data_storage::boolean::{GetDataOwnerResponse, GetValueResponse};
use andromeda_std::{amp::AndrAddr, error::ContractError};
use cosmwasm_std::Storage;

pub fn get_value(storage: &dyn Storage) -> Result<GetValueResponse, ContractError> {
    let value = DATA.load(storage)?;
    Ok(GetValueResponse { value })
}

pub fn get_data_owner(storage: &dyn Storage) -> Result<GetDataOwnerResponse, ContractError> {
    let owner = DATA_OWNER.load(storage)?;
    Ok(GetDataOwnerResponse {
        owner: AndrAddr::from_string(owner),
    })
}
