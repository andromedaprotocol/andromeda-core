use crate::state::{
    generate_assign_app_message, load_component_addresses, load_component_addresses_with_name,
    load_component_descriptors, ADO_ADDRESSES, ADO_DESCRIPTORS, ADO_IDX, APP_NAME, ASSIGNED_IDX,
};
use andromeda_app::app::{
    AppComponent, ComponentAddress, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg,
    QueryMsg,
};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::amp::AndrAddr;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::os::vfs::{convert_component_name, ExecuteMsg as VFSExecuteMsg};
use andromeda_std::{
    ado_base::InstantiateMsg as BaseInstantiateMsg,
    common::{encode_binary, response::get_reply_address},
    error::{from_semver, ContractError},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    Response, StdError, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use crate::execute;
use semver::Version;

pub fn component_address(deps: Deps, name: String) -> Result<String, ContractError> {
    let value = ADO_ADDRESSES.load(deps.storage, &name)?;
    Ok(value.to_string())
}

pub fn component_descriptors(deps: Deps) -> Result<Vec<AppComponent>, ContractError> {
    let value = load_component_descriptors(deps.storage)?;
    Ok(value)
}

pub fn component_exists(deps: Deps, name: String) -> bool {
    ADO_ADDRESSES.has(deps.storage, &name)
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
