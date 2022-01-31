use std::convert::TryInto;

use cosmwasm_std::{
    to_binary, wasm_instantiate, Addr, Api, Binary, CosmosMsg, Event, Order, QuerierWrapper,
    QueryRequest, ReplyOn, StdError, Storage, SubMsg, WasmQuery,
};
use cw_storage_plus::{Bound, Item, Map};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    communication::query_get, error::ContractError, factory::CodeIdResponse, rates::Funds, require,
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
    Receipt,
    /// Used for external contracts, undocumented
    Other,
}

/// Conversion from a module type to string, primarily used to query code ids from our factory contract
impl From<ModuleType> for String {
    fn from(module_type: ModuleType) -> Self {
        match module_type {
            ModuleType::Receipt => String::from("receipt"),
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

/// The type of ADO that is using these modules.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ADOType {
    CW721,
    CW20,
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

    /// Validates `self` by checking that it is unique, does not conflict with any other module,
    /// and does not conflict with the creating ADO.
    pub fn validate(&self, modules: &[Module], ado_type: &ADOType) -> Result<(), ContractError> {
        require(self.is_unique(modules), ContractError::ModuleNotUnique {})?;

        if ado_type == &ADOType::CW20 && contains_module(modules, ModuleType::Auction) {
            return Err(ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string(),
            });
        }

        Ok(())
    }

    /// Determines if `self` is unique within the context of a vector of `Module`
    ///
    /// ## Arguments
    /// * `all_modules` - The vector of modules containing the provided module
    ///
    /// Returns a `boolean` representing whether the module is unique or not
    fn is_unique(&self, all_modules: &[Module]) -> bool {
        let mut total = 0;
        all_modules.iter().for_each(|m| {
            if self == m {
                total += 1;
            }
        });

        total == 1
    }
}

/// Checks if any element of `modules` contains one of type `module_type`.
fn contains_module(modules: &[Module], module_type: ModuleType) -> bool {
    modules.iter().any(|m| m.module_type == module_type)
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

/// Validates all modules.
pub fn validate_modules(modules: &[Module], ado_type: ADOType) -> Result<(), ContractError> {
    for module in modules {
        module.validate(modules, &ado_type)?;
    }

    Ok(())
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
        let mod_resp: Result<T, StdError> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: addr,
            msg: to_binary(&msg)?,
        }));
        if let Ok(mod_resp) = mod_resp {
            resp.push(mod_resp);
        }
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
) -> Result<(Vec<SubMsg>, Vec<Event>, Funds), ContractError> {
    let modules: Vec<ModuleInfoWithAddress> = load_modules_with_address(storage)?;
    let mut remainder = amount;
    let mut msgs: Vec<SubMsg> = Vec::new();
    let mut events: Vec<Event> = Vec::new();
    let mut receipt_module_address: Option<String> = None;
    for module in modules {
        if module.module.module_type == ModuleType::Receipt {
            // If receipt module exists we want to make sure we do it last.
            receipt_module_address = Some(module.address.clone());
            continue;
        }
        let mod_resp = query_on_funds_transfer(
            querier,
            msg.clone(),
            sender.clone(),
            module.address.clone(),
            remainder.clone(),
        );
        if let Ok(mod_resp) = mod_resp {
            remainder = mod_resp.leftover_funds;
            msgs = [msgs, mod_resp.msgs].concat();
            events = [events, mod_resp.events].concat();
        }
    }
    if let Some(receipt_module_address) = receipt_module_address {
        let mod_resp = query_on_funds_transfer(
            querier,
            to_binary(&events)?,
            sender,
            receipt_module_address,
            remainder.clone(),
        )?;
        msgs = [msgs, mod_resp.msgs].concat();
        events = [events, mod_resp.events].concat();
    }

    Ok((msgs, events, remainder))
}

fn query_on_funds_transfer(
    querier: QuerierWrapper,
    payload: Binary,
    sender: String,
    contract_addr: String,
    amount: Funds,
) -> Result<OnFundsTransferResponse, StdError> {
    let query_msg = AndromedaHook::OnFundsTransfer {
        payload,
        sender,
        amount,
    };
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&query_msg)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_addresslist() {
        let addresslist_module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("".to_string()),
        };

        let res = addresslist_module.validate(
            &[addresslist_module.clone(), addresslist_module.clone()],
            &ADOType::CW721,
        );
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let auction_module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("".into()),
        };
        addresslist_module
            .validate(
                &[addresslist_module.clone(), auction_module],
                &ADOType::CW721,
            )
            .unwrap();
    }

    #[test]
    fn test_validate_auction() {
        let module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("".to_string()),
        };

        let res = module.validate(&[module.clone(), module.clone()], &ADOType::CW721);
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let res = module.validate(&[module.clone()], &ADOType::CW20);
        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err()
        );

        let other_module = Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address("".to_string()),
        };
        module
            .validate(&[module.clone(), other_module], &ADOType::CW721)
            .unwrap();
    }

    #[test]
    fn test_validate_rates() {
        let module = Module {
            module_type: ModuleType::Rates,
            instantiate: InstantiateType::Address("".to_string()),
        };

        let res = module.validate(&[module.clone(), module.clone()], &ADOType::CW721);
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let other_module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("".to_string()),
        };
        module
            .validate(&[module.clone(), other_module], &ADOType::CW721)
            .unwrap();
    }

    #[test]
    fn test_validate_receipt() {
        let module = Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address("".to_string()),
        };

        let res = module.validate(&[module.clone(), module.clone()], &ADOType::CW721);
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let other_module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("".to_string()),
        };
        module
            .validate(&[module.clone(), other_module], &ADOType::CW721)
            .unwrap();
    }
}
