use crate::state::{
    load_component_addresses_with_name, load_component_descriptors, ADO_ADDRESSES, APP_NAME,
};
use andromeda_app::app::{AppComponent, ComponentAddress, ComponentExistsResponse, ConfigResponse};
use andromeda_std::ado_contract::ADOContract;

use andromeda_std::error::ContractError;

use cosmwasm_std::Deps;

pub fn component_address(deps: Deps, name: String) -> Result<String, ContractError> {
    let value = ADO_ADDRESSES.load(deps.storage, &name)?;
    Ok(value.to_string())
}

pub fn component_descriptors(deps: Deps) -> Result<Vec<AppComponent>, ContractError> {
    let value = load_component_descriptors(deps.storage)?;
    Ok(value)
}

pub fn component_exists(deps: Deps, name: String) -> ComponentExistsResponse {
    ComponentExistsResponse {
        component_exists: ADO_ADDRESSES.has(deps.storage, &name),
    }
}

pub fn component_addresses_with_name(deps: Deps) -> Result<Vec<ComponentAddress>, ContractError> {
    let value = load_component_addresses_with_name(deps.storage)?;
    Ok(value)
}

pub fn config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let name = APP_NAME.load(deps.storage)?;
    let owner = ADOContract::default().query_contract_owner(deps)?.owner;

    Ok(ConfigResponse { name, owner })
}
