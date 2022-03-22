use crate::ADOContract;

use common::{
    ado_base::query_get,
    encode_binary,
    error::ContractError,
    primitive::{AndromedaContract, GetValueResponse},
};
use cosmwasm_std::{QuerierWrapper, Storage};

impl<'a> ADOContract<'a> {
    pub fn get_address(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        contract: AndromedaContract,
    ) -> Result<String, ContractError> {
        let address = self.primitive_contract.load(storage)?;
        let data = encode_binary(&contract.to_string())?;
        let res: GetValueResponse = query_get(Some(data), address.to_string(), querier)?;
        Ok(res.value.try_get_string()?)
    }
}
