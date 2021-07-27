use crate::whitelist::Whitelist;
use cosmwasm_std::{
    Api, CosmosMsg, Env, Extern, HumanAddr, LogAttribute, Querier, StdResult, Storage,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type Fee = i64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub enum ModuleDefinition {
    WhiteList { moderators: Vec<HumanAddr> },
    Taxable { tax: Fee, receivers: Vec<HumanAddr> },
    // Royalties { fee: Fee, receivers: Vec<HumanAddr> },
}

pub fn as_module(definition: ModuleDefinition) -> impl Module {
    match definition {
        ModuleDefinition::WhiteList { moderators } => Whitelist { moderators },
        ModuleDefinition::Taxable { .. } => Whitelist { moderators: vec![] },
    }
}

pub fn as_modules(definitions: Vec<ModuleDefinition>) -> Vec<impl Module> {
    definitions.into_iter().map(|d| as_module(d)).collect()
}

#[derive(Debug, PartialEq)]
pub struct HookResponse {
    pub msgs: Vec<CosmosMsg>,
    pub logs: Vec<LogAttribute>,
}

impl HookResponse {
    pub fn default() -> Self {
        HookResponse {
            msgs: vec![],
            logs: vec![],
        }
    }
}

pub trait Module {
    fn validate(&self, extensions: Vec<ModuleDefinition>) -> StdResult<bool>;
    fn as_definition(&self) -> ModuleDefinition;
    fn pre_publish<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
}
