use crate::state::ADOContract;
use andromeda_protocol::{
    ado_base::{
        operators::{IsOperatorResponse, OperatorsResponse},
        ownership::ContractOwnerResponse,
        AndromedaQuery,
    },
    communication::{encode_binary, parse_message},
    error::ContractError,
    require,
};
use cosmwasm_std::{Binary, Deps, Env, Order};
use serde::de::DeserializeOwned;

type QueryFunction<Q> = fn(Deps, Env, Q) -> Result<Binary, ContractError>;

impl<'a> ADOContract<'a> {
    pub fn query<Q: DeserializeOwned>(
        &self,
        deps: Deps,
        env: Env,
        msg: AndromedaQuery,
        query_function: QueryFunction<Q>,
    ) -> Result<Binary, ContractError> {
        match msg {
            AndromedaQuery::Get(data) => {
                require(!self.is_nested(&data), ContractError::NestedAndromedaMsg {})?;
                let received: Q = parse_message(&data)?;
                (query_function)(deps, env, received)
            }
            AndromedaQuery::Owner {} => encode_binary(&self.query_contract_owner(deps)?),
            AndromedaQuery::Operators {} => encode_binary(&self.query_operators(deps)?),
            AndromedaQuery::IsOperator { address } => {
                encode_binary(&self.query_is_operator(deps, &address)?)
            }
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
        let operators: Result<Vec<String>, ContractError> = self
            .operators
            .keys(deps.storage, None, None, Order::Ascending)
            .map(|k| {
                String::from_utf8(k).map_err(|_| ContractError::ParsingError {
                    err: "parsing escrow key".to_string(),
                })
            })
            .collect();
        Ok(OperatorsResponse {
            operators: operators?,
        })
    }
}
