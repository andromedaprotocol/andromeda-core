use crate::ado_contract::ADOContract;
use crate::error::ContractError;
use crate::os::aos_querier::AOSQuerier;
use crate::os::kernel::QueryMsg as KernelQueryMsg;
use cosmwasm_std::{Addr, Binary, CosmosMsg, QuerierWrapper, ReplyOn, Storage, SubMsg, WasmMsg};

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

    /// Gets the address for `contract` stored in the primitive contract.
    pub fn get_address_from_kernel(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        contract: &str,
    ) -> Result<Addr, ContractError> {
        let kernel_address = self.kernel_address.load(storage)?;
        let query = KernelQueryMsg::KeyAddress {
            key: contract.to_string(),
        };
        let address: Addr = querier.query_wasm_smart(kernel_address, &query)?;

        Ok(address)
    }

    fn get_code_id(
        &self,
        storage: &mut dyn Storage,
        querier: &QuerierWrapper,
        name: &str,
    ) -> Result<u64, ContractError> {
        // Do we want to cache the factory address?
        let adodb_addr = self.get_adodb_address(storage, querier)?;
        let code_id: u64 = AOSQuerier::code_id_getter(querier, &adodb_addr, name)?;
        Ok(code_id)
    }
}
