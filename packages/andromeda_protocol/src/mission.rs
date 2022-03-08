use crate::{
    communication::{AndromedaMsg, AndromedaQuery},
    error::ContractError,
    factory::get_ado_codeid,
};
use cosmwasm_std::{Binary, CosmosMsg, QuerierWrapper, ReplyOn, Storage, SubMsg, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MissionComponent {
    pub name: String,
    pub ado_type: String,
    pub instantiate_msg: Binary,
}

// DEV NOTE: Redundant with CW721 modules, possibly merge the two implementations? Or maybe parts of it?
/// A mission component is an ADO that is used in the flow of the mission
impl MissionComponent {
    /// Generates an instantiation message for the given Mission Component
    /// Attaches the vector index of the Mission Component in order to map the Mission Component's name to its instantiated address
    pub fn generate_instantiate_msg(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper,
        idx: u64,
    ) -> Result<SubMsg, ContractError> {
        match get_ado_codeid(storage, querier, &self.ado_type)? {
            None => Err(ContractError::InvalidModule {
                msg: Some(String::from(
                    "ADO type provided does not have a valid Code Id",
                )),
            }),
            Some(code_id) => Ok(SubMsg {
                id: idx,
                reply_on: ReplyOn::Always,
                msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: None,
                    code_id,
                    msg: self.instantiate_msg.clone(),
                    funds: vec![],
                    label: format!("Instantiate ADO: {}", self.ado_type.clone()),
                }),
                gas_limit: None,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub operators: Vec<String>,
    pub mission: Vec<MissionComponent>,
    pub xfer_ado_ownership: bool,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    AddMissionComponent { component: MissionComponent },
    ClaimOwnership { name: Option<String> },
    ProxyMessage { name: String, msg: Binary },
    UpdateAddress { name: String, addr: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AndrQuery(AndromedaQuery),
    GetAddress { name: String },
    GetComponents {},
    GetAddresses {},
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    // use super::*;
}
