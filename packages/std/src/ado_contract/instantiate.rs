use crate::ado_contract::ADOContract;
use crate::{ado_base::query_get, encode_binary, error::ContractError};
use cosmwasm_std::{Binary, CosmosMsg, QuerierWrapper, ReplyOn, Storage, SubMsg, WasmMsg};

impl<'a> ADOContract<'a> {
    pub fn generate_instantiate_msg(
        &self,
        storage: &mut dyn Storage,
        querier: &QuerierWrapper,
        msg_id: u64,
        msg: Binary,
        ado_type: String,
        sender: String,
    ) -> Result<SubMsg, ContractError> {
        match self.get_code_id(storage, querier, &ado_type) {
            Err(_) => Err(ContractError::InvalidModule {
                msg: Some(String::from(
                    "ADO type provided does not have a valid Code Id",
                )),
            }),
            Ok(code_id) => Ok(SubMsg {
                id: msg_id,
                reply_on: ReplyOn::Always,
                msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: Some(sender),
                    code_id,
                    msg,
                    funds: vec![],
                    label: format!("Instantiate: {ado_type}"),
                }),
                gas_limit: None,
            }),
        }
    }

    fn get_code_id(
        &self,
        storage: &mut dyn Storage,
        querier: &QuerierWrapper,
        name: &str,
    ) -> Result<u64, ContractError> {
        // Do we want to cache the factory address?
        let adodb_addr = self.get_address_from_kernel(storage, querier, "adodb")?;
        let code_id: u64 = query_get(Some(encode_binary(&name)?), adodb_addr.to_string(), querier)?;
        Ok(code_id)
    }
}
