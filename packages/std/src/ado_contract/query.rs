use crate::ado_contract::state::ADOContract;
use crate::{
    ado_base::{
        ado_type::TypeResponse,
        block_height::BlockHeightResponse,
        kernel_address::KernelAddressResponse,
        ownership::{ContractOwnerResponse, PublisherResponse},
        version::VersionResponse,
        AndromedaQuery,
    },
    common::encode_binary,
    error::ContractError,
};
use cosmwasm_std::{from_binary, to_binary, Binary, Deps, Env};
use cw2::get_contract_version;
use serde::Serialize;

impl<'a> ADOContract<'a> {
    #[allow(unreachable_patterns)]
    pub fn query(
        &self,
        deps: Deps,
        _env: Env,
        msg: impl Serialize,
    ) -> Result<Binary, ContractError> {
        let msg = to_binary(&msg)?;

        match from_binary::<AndromedaQuery>(&msg) {
            Ok(msg) => match msg {
                AndromedaQuery::Owner {} => encode_binary(&self.query_contract_owner(deps)?),
                AndromedaQuery::OriginalPublisher {} => {
                    encode_binary(&self.query_original_publisher(deps)?)
                }
                AndromedaQuery::Type {} => encode_binary(&self.query_type(deps)?),
                AndromedaQuery::BlockHeightUponCreation {} => {
                    encode_binary(&self.query_block_height_upon_creation(deps)?)
                }
                AndromedaQuery::KernelAddress {} => {
                    encode_binary(&self.query_kernel_address(deps)?)
                }
                AndromedaQuery::Version {} => encode_binary(&self.query_version(deps)?),
                #[cfg(feature = "modules")]
                AndromedaQuery::Module { id } => encode_binary(&self.query_module(deps, id)?),
                #[cfg(feature = "modules")]
                AndromedaQuery::ModuleIds {} => encode_binary(&self.query_module_ids(deps)?),
                AndromedaQuery::AppContract {} => {
                    encode_binary(&self.get_app_contract(deps.storage)?)
                }
                AndromedaQuery::Permissions {
                    actor,
                    limit,
                    start_after,
                } => encode_binary(&self.query_permissions(deps, actor, limit, start_after)?),
                AndromedaQuery::PermissionedActions {} => {
                    encode_binary(&self.query_permissioned_actions(deps)?)
                }
                _ => Err(ContractError::UnsupportedOperation {}),
            },
            Err(_) => Err(ContractError::UnsupportedOperation {}),
        }
    }
}

impl<'a> ADOContract<'a> {
    #[inline]
    pub fn query_contract_owner(&self, deps: Deps) -> Result<ContractOwnerResponse, ContractError> {
        let owner = self.owner.load(deps.storage)?;

        Ok(ContractOwnerResponse {
            owner: owner.to_string(),
        })
    }

    #[inline]
    pub fn query_kernel_address(&self, deps: Deps) -> Result<KernelAddressResponse, ContractError> {
        let kernel_address = self.kernel_address.load(deps.storage)?;
        Ok(KernelAddressResponse { kernel_address })
    }

    #[inline]
    pub fn query_original_publisher(&self, deps: Deps) -> Result<PublisherResponse, ContractError> {
        let original_publisher = self.original_publisher.load(deps.storage)?.to_string();
        Ok(PublisherResponse { original_publisher })
    }

    #[inline]
    pub fn query_block_height_upon_creation(
        &self,
        deps: Deps,
    ) -> Result<BlockHeightResponse, ContractError> {
        let block_height = self.block_height.load(deps.storage)?;
        Ok(BlockHeightResponse { block_height })
    }

    #[inline]
    pub fn query_type(&self, deps: Deps) -> Result<TypeResponse, ContractError> {
        let ado_type = self.ado_type.load(deps.storage)?;
        Ok(TypeResponse { ado_type })
    }

    #[inline]
    pub fn query_version(&self, deps: Deps) -> Result<VersionResponse, ContractError> {
        let contract_version = get_contract_version(deps.storage)?;
        Ok(VersionResponse {
            version: contract_version.version,
        })
    }
}
