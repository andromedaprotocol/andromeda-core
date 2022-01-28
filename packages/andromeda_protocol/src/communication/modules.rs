use std::convert::TryInto;

use cosmwasm_std::{
    to_binary, wasm_instantiate, Addr, Api, Binary, CosmosMsg, Order, QuerierWrapper, QueryRequest,
    ReplyOn, Storage, SubMsg, WasmQuery,
};
use cw_storage_plus::{Bound, Item, Map};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    communication::query_get, error::ContractError, factory::CodeIdResponse, rates::Funds,
};

use super::hooks::{AndromedaHook, OnFundsTransferResponse};

pub const FACTORY_ADDRESS: &str = "terra1...";
pub const MODULE_INFO: Map<String, Module> = Map::new("andr_modules");
pub const MODULE_ADDR: Map<String, Addr> = Map::new("andr_module_addresses");
pub const MODULE_IDX: Item<u64> = Item::new("andr_module_idx");

/// An enum describing the different available modules for any Andromeda Token contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModuleType {
    Rates,
    AddressList,
    Auction,
    /// Used for external contracts, undocumented
    Other,
}

/// Conversion from a module type to string, primarily used to query code ids from our factory contract
impl From<ModuleType> for String {
    fn from(module_type: ModuleType) -> Self {
        match module_type {
            ModuleType::AddressList => String::from("address_list"),
            ModuleType::Rates => String::from("rates"),
            ModuleType::Auction => String::from("auction"),
            ModuleType::Other => String::from("other"),
        }
    }
}

/// Modules can be instantiated in two different ways
/// New - Provide an instantiation message for the contract, a new contract will be instantiated and the address recorded
/// Address - Provide an address for an already instantiated module contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstantiateType {
    New(Binary),
    Address(String),
}

/// A struct describing a token module, provided with the instantiation message this struct is used to record the info about the module and how/if it should be instantiated
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Module {
    module_type: ModuleType,
    instantiate: InstantiateType,
}

/// Struct used to represent a module and its currently recorded address
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ModuleInfoWithAddress {
    module: Module,
    address: String,
}

impl Module {
    /// Queries the code id for a module from the factory contract
    pub fn get_code_id(&self, querier: QuerierWrapper) -> Result<Option<u64>, ContractError> {
        match self.module_type {
            ModuleType::Other => Ok(None),
            _ => {
                let code_id_resp: CodeIdResponse = query_get(
                    Some(to_binary(&String::from(self.module_type.clone()))?),
                    FACTORY_ADDRESS.to_string(),
                    querier,
                )?;
                Ok(Some(code_id_resp.code_id))
            }
        }
    }

    /// Generate an instantiation message for the module if its required
    pub fn generate_instantiate_msg(
        &self,
        querier: QuerierWrapper,
        module_id: u64,
    ) -> Result<Option<SubMsg>, ContractError> {
        match self.instantiate.clone() {
            InstantiateType::New(msg) => {
                match self.get_code_id(querier)? {
                    None => Err(ContractError::InvalidModule {
                        msg: Some(String::from(
                            "Module type provided does not have a valid Code Id",
                        )),
                    }),
                    Some(code_id) => Ok(Some(SubMsg {
                        id: module_id, //TODO: ADD ID,
                        reply_on: ReplyOn::Always,
                        msg: CosmosMsg::Wasm(wasm_instantiate(
                            code_id,
                            &msg,
                            vec![],
                            format!("Instantiate: {}", String::from(self.module_type.clone())),
                        )?),
                        gas_limit: None,
                    })),
                }
            }
            _ => Ok(None),
        }
    }
}

/// Registers a module
/// If the module has provided an address as its form of instantiation this address is recorded
/// Each module is assigned a u64 index so as it can be unregistered/altered
/// The assigned u64 index is used as the message id for use in the `reply` entry point of the contract
pub fn register_module(
    storage: &mut dyn Storage,
    api: &dyn Api,
    module: &Module,
) -> Result<u64, ContractError> {
    let idx = match MODULE_IDX.load(storage) {
        Ok(index) => index,
        Err(..) => 1u64,
    };
    MODULE_INFO.save(storage, idx.to_string(), module)?;
    MODULE_IDX.save(storage, &(idx + 1))?;
    if let InstantiateType::Address(addr) = module.instantiate.clone() {
        MODULE_ADDR.save(storage, idx.to_string(), &api.addr_validate(&addr)?)?;
    }

    Ok(idx)
}

/// Loads all registered modules in Vector form
pub fn load_modules(storage: &dyn Storage) -> Result<Vec<Module>, ContractError> {
    let module_idx = match MODULE_IDX.load(storage) {
        Ok(index) => index,
        Err(..) => 1,
    };
    let min = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    // let max = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    let modules: Vec<Module> = MODULE_INFO
        .range(storage, min, None, Order::Ascending)
        .take(module_idx.try_into().unwrap())
        .flatten()
        .map(|(_vec, module)| module)
        .collect();

    Ok(modules)
}

/// Loads all registered module addresses in Vector form
pub fn load_module_addresses(storage: &dyn Storage) -> Result<Vec<String>, ContractError> {
    let module_idx = match MODULE_IDX.load(storage) {
        Ok(index) => index,
        Err(..) => 1,
    };
    let min = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    // let max = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    let module_addresses: Vec<String> = MODULE_ADDR
        .range(storage, min, None, Order::Ascending)
        .take(module_idx.try_into().unwrap())
        .flatten()
        .map(|(_vec, addr)| addr.to_string())
        .collect();

    Ok(module_addresses)
}

/// Loads all modules with their registered addresses in Vector form
pub fn load_modules_with_address(
    storage: &dyn Storage,
) -> Result<Vec<ModuleInfoWithAddress>, ContractError> {
    let modules = load_modules(storage)?;
    let module_idx = match MODULE_IDX.load(storage) {
        Ok(index) => index,
        Err(..) => 1,
    };
    let min = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    // let max = Some(Bound::Inclusive(1u64.to_le_bytes().to_vec()));
    let module_addresses: Vec<String> = MODULE_ADDR
        .range(storage, min, None, Order::Ascending)
        .take(module_idx.try_into().unwrap())
        .flatten()
        .map(|(_vec, addr)| addr.to_string())
        .collect();

    let mut modules_with_addresses: Vec<ModuleInfoWithAddress> = Vec::new();
    for (index, module_address) in module_addresses.iter().enumerate() {
        let module_opt = modules.get(index);
        if let Some(module) = module_opt {
            modules_with_addresses.push(ModuleInfoWithAddress {
                module: module.clone(),
                address: module_address.to_string(),
            });
        }
    }

    Ok(modules_with_addresses)
}

/// Sends the provided hook message to all registered modules
pub fn module_hook<T>(
    storage: &dyn Storage,
    querier: QuerierWrapper,
    msg: AndromedaHook,
) -> Result<Vec<T>, ContractError>
where
    T: DeserializeOwned,
{
    let addresses: Vec<String> = load_module_addresses(storage)?;
    let mut resp: Vec<T> = Vec::new();
    for addr in addresses {
        let mod_resp: T = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: addr,
            msg: to_binary(&msg)?,
        }))?;
        resp.push(mod_resp);
    }

    Ok(resp)
}

/// Sends the provided hook message to all registered modules
pub fn on_funds_transfer(
    storage: &dyn Storage,
    querier: QuerierWrapper,
    sender: String,
    amount: Funds,
    msg: Binary,
) -> Result<(Vec<SubMsg>, Funds), ContractError> {
    let addresses: Vec<String> = load_module_addresses(storage)?;
    let mut remainder = amount;
    let mut msgs: Vec<SubMsg> = Vec::new();
    for addr in addresses {
        let query_msg = AndromedaHook::OnFundsTransfer {
            msg: msg.clone(),
            sender: sender.clone(),
            amount: remainder,
        };
        let mod_resp: OnFundsTransferResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: addr,
                msg: to_binary(&query_msg)?,
            }))?;
        remainder = mod_resp.leftover_funds;
        msgs = [msgs, mod_resp.msgs].concat();
    }

    Ok((msgs, remainder))
}
