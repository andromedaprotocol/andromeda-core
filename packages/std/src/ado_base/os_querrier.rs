use super::{
    ado_type::TypeResponse, kernel_address::KernelAddressResponse,
    ownership::ContractOwnerResponse, version::VersionResponse,
};
use crate::{ado_contract::ADOContract, error::ContractError};
use cosmwasm_std::Deps;

pub fn version(deps: Deps) -> Result<VersionResponse, ContractError> {
    ADOContract::default().query_version(deps)
}

// Can't name it type because it's a keyword in Rust
pub fn ado_type(deps: Deps) -> Result<TypeResponse, ContractError> {
    ADOContract::default().query_type(deps)
}

pub fn owner(deps: Deps) -> Result<ContractOwnerResponse, ContractError> {
    ADOContract::default().query_contract_owner(deps)
}

pub fn kernel_address(deps: Deps) -> Result<KernelAddressResponse, ContractError> {
    ADOContract::default().query_kernel_address(deps)
}
