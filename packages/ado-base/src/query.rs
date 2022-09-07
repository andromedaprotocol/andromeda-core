use crate::state::ADOContract;
use common::{
    ado_base::{
        ado_type::TypeResponse,
        block_height::BlockHeightResponse,
        operators::{IsOperatorResponse, OperatorsResponse},
        ownership::{ContractOwnerResponse, PublisherResponse},
        version::VersionResponse,
        AndromedaQuery, QueryMsg,
    },
    encode_binary,
    error::ContractError,
    parse_message, require,
};
use cosmwasm_std::{Binary, Deps, Env, Order};
use serde::de::DeserializeOwned;

type QueryFunction<Q> = fn(Deps, Env, Q) -> Result<Binary, ContractError>;

impl<'a> ADOContract<'a> {
    #[allow(unreachable_patterns)]
    pub fn query<Q: DeserializeOwned>(
        &self,
        deps: Deps,
        env: Env,
        msg: AndromedaQuery,
        query_function: QueryFunction<Q>,
    ) -> Result<Binary, ContractError> {
        match msg {
            AndromedaQuery::Get(data) => {
                require(
                    !self.is_nested::<QueryMsg>(&data),
                    ContractError::NestedAndromedaMsg {},
                )?;
                let received: Q = parse_message(&data)?;
                (query_function)(deps, env, received)
            }
            AndromedaQuery::Owner {} => encode_binary(&self.query_contract_owner(deps)?),
            AndromedaQuery::Operators {} => encode_binary(&self.query_operators(deps)?),
            AndromedaQuery::OriginalPublisher {} => {
                encode_binary(&self.query_original_publisher(deps)?)
            }
            AndromedaQuery::Type {} => encode_binary(&self.query_type(deps)?),
            AndromedaQuery::BlockHeightUponCreation {} => {
                encode_binary(&self.query_block_height_upon_creation(deps)?)
            }
            AndromedaQuery::IsOperator { address } => {
                encode_binary(&self.query_is_operator(deps, &address)?)
            }
            AndromedaQuery::Version {} => encode_binary(&self.query_version(deps)?),
            #[cfg(feature = "modules")]
            AndromedaQuery::Module { id } => encode_binary(&self.query_module(deps, id)?),
            #[cfg(feature = "modules")]
            AndromedaQuery::ModuleIds {} => encode_binary(&self.query_module_ids(deps)?),
            _ => Err(ContractError::UnsupportedOperation {}),
        }
    }
}

impl<'a> ADOContract<'a> {
    pub fn query_contract_owner(&self, deps: Deps) -> Result<ContractOwnerResponse, ContractError> {
        let owner = self.owner.load(deps.storage)?;

        Ok(ContractOwnerResponse {
            owner: owner.to_string(),
        })
    }

    pub fn query_is_operator(
        &self,
        deps: Deps,
        addr: &str,
    ) -> Result<IsOperatorResponse, ContractError> {
        Ok(IsOperatorResponse {
            is_operator: self.operators.has(deps.storage, addr),
        })
    }

    pub fn query_operators(&self, deps: Deps) -> Result<OperatorsResponse, ContractError> {
        let operators: Result<Vec<String>, _> = self
            .operators
            .keys(deps.storage, None, None, Order::Ascending)
            .collect();
        Ok(OperatorsResponse {
            operators: operators?,
        })
    }
    pub fn query_original_publisher(&self, deps: Deps) -> Result<PublisherResponse, ContractError> {
        let original_publisher = self.original_publisher.load(deps.storage)?.to_string();
        Ok(PublisherResponse { original_publisher })
    }

    pub fn query_block_height_upon_creation(
        &self,
        deps: Deps,
    ) -> Result<BlockHeightResponse, ContractError> {
        let block_height = self.block_height.load(deps.storage)?;
        Ok(BlockHeightResponse { block_height })
    }

    pub fn query_type(&self, deps: Deps) -> Result<TypeResponse, ContractError> {
        let ado_type = self.ado_type.load(deps.storage)?;
        Ok(TypeResponse { ado_type })
    }

    pub fn query_version(&self, deps: Deps) -> Result<VersionResponse, ContractError> {
        let version = self.version.load(deps.storage)?;
        Ok(VersionResponse { version })
    }
}
