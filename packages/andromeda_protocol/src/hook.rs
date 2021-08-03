use cosmwasm_std::{Binary, CosmosMsg, StdResult, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitHook {
    pub msg: Binary,
    pub contract_addr: String,
}

impl InitHook {
    pub fn into_cosmos_msg(self) -> StdResult<CosmosMsg> {
        let execute = WasmMsg::Execute {
            contract_addr: self.contract_addr,
            msg: self.msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}
