use crate::ADOContract;

use common::{
    ado_base::query_get,
    encode_binary,
    error::ContractError,
    primitive::{AndromedaContract, GetValueResponse},
};
use cosmwasm_std::{Binary, CosmosMsg, QuerierWrapper, ReplyOn, Storage, SubMsg, WasmMsg};

impl<'a> ADOContract<'a> {
    pub fn generate_instantiate_msg(
        &self,
        storage: &dyn Storage,
        querier: QuerierWrapper,
        module_id: u64,
        msg: Binary,
        name: String,
    ) -> Result<SubMsg, ContractError> {
        match self.get_code_id(storage, querier, &name)? {
            None => Err(ContractError::InvalidModule {
                msg: Some(String::from(
                    "Module type provided does not have a valid Code Id",
                )),
            }),
            Some(code_id) => Ok(SubMsg {
                id: module_id,
                reply_on: ReplyOn::Always,
                msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: None,
                    code_id,
                    msg,
                    funds: vec![],
                    label: format!("Instantiate: {}", name),
                }),
                gas_limit: None,
            }),
        }
    }

    fn get_code_id(
        &self,
        storage: &dyn Storage,
        querier: QuerierWrapper,
        name: &str,
    ) -> Result<Option<u64>, ContractError> {
        let factory_address = self.get_address(storage, querier, AndromedaContract::Factory)?;
        let code_id: u64 = query_get(Some(encode_binary(&name)?), factory_address, &querier)?;
        Ok(Some(code_id))
    }

    fn get_address(
        &self,
        storage: &dyn Storage,
        querier: QuerierWrapper,
        contract: AndromedaContract,
    ) -> Result<String, ContractError> {
        let address = self.primitive_contract.load(storage)?;
        let data = encode_binary(&contract.to_string())?;
        let res: GetValueResponse = query_get(Some(data), address.to_string(), &querier)?;
        Ok(res.value.try_get_string()?)
    }
}
