use crate::whitelist::Whitelist;
use cosmwasm_std::{
    Api, Coin, CosmosMsg, Env, Extern, HumanAddr, LogAttribute, Querier, StdResult, Storage,
};
use cosmwasm_storage::{singleton, singleton_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const KEY_MODULES: &[u8] = b"modules";

pub type Fee = i64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub enum ModuleDefinition {
    WhiteList { moderators: Vec<HumanAddr> },
    Taxable { tax: Fee, receivers: Vec<HumanAddr> },
    // Royalties { fee: Fee, receivers: Vec<HumanAddr> },
}

//Converts a ModuleDefinition to a Module struct
pub fn as_module(definition: ModuleDefinition) -> impl Module {
    match definition {
        ModuleDefinition::WhiteList { moderators } => Whitelist { moderators },
        ModuleDefinition::Taxable { .. } => Whitelist { moderators: vec![] },
    }
}

//Converts a vector of ModuleDefinitions to a vector of Module structs
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
    fn pre_handle<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_publish<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_transfer<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
        _from: HumanAddr,
        _to: HumanAddr,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_transfer_agreement<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
        _amount: Coin,
        _buyer: HumanAddr,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_burn<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
    fn pre_archive<S: Storage, A: Api, Q: Querier>(
        &self,
        _deps: &mut Extern<S, A, Q>,
        _env: Env,
        _token_id: i64,
    ) -> StdResult<HookResponse> {
        Ok(HookResponse::default())
    }
}

pub fn store_modules<S: Storage>(
    storage: &mut S,
    module_defs: Vec<ModuleDefinition>,
) -> StdResult<()> {
    //Validate each module before storing
    let modules = as_modules(module_defs.clone());
    for module in modules {
        module.validate(module_defs.clone())?;
    }

    singleton(storage, KEY_MODULES).save(&module_defs)
}

pub fn read_modules<S: Storage>(storage: &S) -> StdResult<Vec<impl Module>> {
    let module_defs = singleton_read(storage, KEY_MODULES).load()?;

    Ok(as_modules(module_defs))
}
