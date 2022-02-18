use std::convert::TryInto;

use cosmwasm_std::{
    to_binary, Addr, Api, Binary, CosmosMsg, DepsMut, Event, MessageInfo, Order,
    QuerierWrapper, QueryRequest, ReplyOn, Response, StdError, Storage, SubMsg, Uint64, WasmMsg,
    WasmQuery,
};
use cw_storage_plus::{Bound, Item, Map};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    communication::{query_get, HookMsg},
    error::ContractError,
    factory::CodeIdResponse,
    operators::is_operator,
    ownership::is_contract_owner,
    rates::Funds,
    require,
};

use super::hooks::{AndromedaHook, OnFundsTransferResponse};

pub const FACTORY_ADDRESS: &str = "terra1...";
pub const MODULE_INFO: Map<&str, Module> = Map::new("andr_modules");
pub const MODULE_ADDR: Map<&str, Addr> = Map::new("andr_module_addresses");
// Module type -> module id
pub const MODULE_IDXS: Map<&str, String> = Map::new("module_idxs");
pub const MODULE_IDX: Item<u64> = Item::new("andr_module_idx");

/// An enum describing the different available modules for any Andromeda Token contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModuleType {
    Rates,
    Offers,
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
            ModuleType::Offers => String::from("offers"),
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
    pub module_type: ModuleType,
    pub instantiate: InstantiateType,
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
        if let InstantiateType::New(msg) = &self.instantiate {
            match self.get_code_id(querier)? {
                None => Err(ContractError::InvalidModule {
                    msg: Some(String::from(
                        "Module type provided does not have a valid Code Id",
                    )),
                }),
                Some(code_id) => Ok(Some(SubMsg {
                    id: module_id, //TODO: ADD ID,
                    reply_on: ReplyOn::Always,
                    msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                        admin: None,
                        code_id,
                        msg: msg.clone(),
                        funds: vec![],
                        label: format!("Instantiate: {}", String::from(self.module_type.clone())),
                    }),
                    gas_limit: None,
                })),
            }
        } else {
            Ok(None)
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
fn register_module(
    storage: &mut dyn Storage,
    api: &dyn Api,
    module: &Module,
) -> Result<u64, ContractError> {
    let idx = match MODULE_IDX.load(storage) {
        Ok(index) => index,
        Err(..) => 1u64,
    };
    let idx_str = idx.to_string();
    MODULE_INFO.save(storage, &idx_str, module)?;
    MODULE_IDX.save(storage, &(idx + 1))?;
    if let InstantiateType::Address(addr) = &module.instantiate {
        MODULE_ADDR.save(storage, &idx_str, &api.addr_validate(addr)?)?;
    }
    MODULE_IDXS.save(storage, &String::from(module.module_type.clone()), &idx_str)?;

    Ok(idx)
}

/// Deregisters a module.
fn deregister_module(storage: &mut dyn Storage, idx: Uint64) -> Result<(), ContractError> {
    let idx_str = idx.to_string();
    if !MODULE_INFO.has(storage, &idx_str) {
        return Err(ContractError::ModuleDoesNotExist {});
    }
    MODULE_INFO.remove(storage, &idx_str);
    MODULE_ADDR.remove(storage, &idx_str);

    Ok(())
}

/// Alters a module
/// If the module has provided an address as its form of instantiation this address is recorded
/// Each module is assigned a u64 index so as it can be unregistered/altered
/// The assigned u64 index is used as the message id for use in the `reply` entry point of the contract
fn alter_module(
    storage: &mut dyn Storage,
    api: &dyn Api,
    idx: Uint64,
    module: &Module,
) -> Result<(), ContractError> {
    let idx_str = idx.to_string();
    if !MODULE_INFO.has(storage, &idx_str) {
        return Err(ContractError::ModuleDoesNotExist {});
    }
    MODULE_INFO.save(storage, &idx_str, module)?;
    if let InstantiateType::Address(addr) = &module.instantiate {
        MODULE_ADDR.save(storage, &idx_str, &api.addr_validate(addr)?)?;
    }
    Ok(())
}

/// A wrapper for `fn register_module`. The parameters are "extracted" from `DepsMut` to be able to
/// execute this in a loop without cloning.
pub fn execute_register_module(
    querier: &QuerierWrapper,
    storage: &mut dyn Storage,
    api: &dyn Api,
    sender: &str,
    module: &Module,
    ado_type: ADOType,
    should_validate: bool,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(storage, sender)? || is_operator(storage, sender)?,
        ContractError::Unauthorized {},
    )?;
    let mut resp = Response::default();
    let idx = register_module(storage, api, module)?;
    if let Some(inst_msg) = module.generate_instantiate_msg(*querier, idx)? {
        resp = resp.add_submessage(inst_msg);
    }
    if should_validate {
        validate_modules(&load_modules(storage)?, ado_type)?;
    }
    Ok(resp.add_attribute("action", "register_module"))
}

/// A wrapper for `fn alter_module`.
pub fn execute_alter_module(
    deps: DepsMut,
    info: MessageInfo,
    module_idx: Uint64,
    module: &Module,
    ado_type: ADOType,
) -> Result<Response, ContractError> {
    let addr = info.sender.as_str();
    require(
        is_contract_owner(deps.storage, addr)? || is_operator(deps.storage, addr)?,
        ContractError::Unauthorized {},
    )?;
    let mut resp = Response::default();
    alter_module(deps.storage, deps.api, module_idx, module)?;
    if let Some(inst_msg) = module.generate_instantiate_msg(deps.querier, module_idx.u64())? {
        resp = resp.add_submessage(inst_msg);
    }
    validate_modules(&load_modules(deps.storage)?, ado_type)?;
    Ok(resp
        .add_attribute("action", "alter_module")
        .add_attribute("module_idx", module_idx))
}

/// A wrapper for `fn deregister_module`.
pub fn execute_deregister_module(
    deps: DepsMut,
    info: MessageInfo,
    module_idx: Uint64,
) -> Result<Response, ContractError> {
    let addr = info.sender.as_str();
    require(
        is_contract_owner(deps.storage, addr)? || is_operator(deps.storage, addr)?,
        ContractError::Unauthorized {},
    )?;
    deregister_module(deps.storage, module_idx)?;
    Ok(Response::default()
        .add_attribute("action", "deregister_module")
        .add_attribute("module_idx", module_idx))
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
    hook_msg: AndromedaHook,
) -> Result<Vec<T>, ContractError>
where
    T: DeserializeOwned,
{
    let addresses: Vec<String> = load_module_addresses(storage)?;
    let mut resp: Vec<T> = Vec::new();
    for addr in addresses {
        let mod_resp: Option<T> = hook_query(querier, hook_msg.clone(), addr)?;
        if let Some(mod_resp) = mod_resp {
            resp.push(mod_resp);
        }
    }

    Ok(resp)
}

/// Queriers the given address with the given hook message and returns the processed result.
fn hook_query<T>(
    querier: QuerierWrapper,
    hook_msg: AndromedaHook,
    addr: String,
) -> Result<Option<T>, ContractError>
where
    T: DeserializeOwned,
{
    let msg = HookMsg::AndrHook(hook_msg);
    let mod_resp: Result<T, StdError> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: addr,
        msg: to_binary(&msg)?,
    }));
    process_module_response(mod_resp)
}

/// Processes the given module response by hiding the error if it is `UnsupportedOperation` and
/// bubbling up any other one. A return value of Ok(None) signifies that the operation was not
/// supported.
fn process_module_response<T>(mod_resp: Result<T, StdError>) -> Result<Option<T>, ContractError> {
    match mod_resp {
        Ok(mod_resp) => Ok(Some(mod_resp)),
        Err(StdError::GenericErr { msg }) => {
            if msg.contains("UnsupportedOperation") {
                Ok(None)
            } else {
                Err(ContractError::Std(StdError::GenericErr { msg }))
            }
        }
        Err(e) => Err(e.into()),
    }
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
        let mod_resp: Option<OnFundsTransferResponse> = hook_query(
            querier,
            AndromedaHook::OnFundsTransfer {
                payload: msg.clone(),
                sender: sender.clone(),
                amount: remainder.clone(),
            },
            module.address.clone(),
        )?;
        if let Some(mod_resp) = mod_resp {
            remainder = mod_resp.leftover_funds;
            msgs = [msgs, mod_resp.msgs].concat();
            events = [events, mod_resp.events].concat();
        }
    }
    if let Some(receipt_module_address) = receipt_module_address {
        let mod_resp: Option<OnFundsTransferResponse> = hook_query(
            querier,
            AndromedaHook::OnFundsTransfer {
                payload: to_binary(&events)?,
                sender,
                amount: remainder.clone(),
            },
            receipt_module_address,
        )?;
        if let Some(mod_resp) = mod_resp {
            msgs = [msgs, mod_resp.msgs].concat();
            events = [events, mod_resp.events].concat();
        }
    }

    Ok((msgs, events, remainder))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ownership::CONTRACT_OWNER;
    use cosmwasm_std::testing::{mock_dependencies, mock_info};

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

    #[test]
    fn test_execute_register_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
        };
        let deps_mut = deps.as_mut();
        CONTRACT_OWNER
            .save(deps_mut.storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_register_module(
            &deps_mut.querier,
            deps_mut.storage,
            deps_mut.api,
            "sender",
            &module,
            ADOType::CW20,
            true,
        );

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_register_module_addr() {
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
        };
        let deps_mut = deps.as_mut();
        CONTRACT_OWNER
            .save(deps_mut.storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_register_module(
            &deps_mut.querier,
            deps_mut.storage,
            deps_mut.api,
            "owner",
            &module,
            ADOType::CW20,
            true,
        )
        .unwrap();

        assert_eq!(
            Response::default().add_attribute("action", "register_module"),
            res
        );

        assert_eq!(
            module,
            MODULE_INFO.load(deps.as_mut().storage, "1").unwrap()
        );

        assert_eq!(
            "address".to_string(),
            MODULE_ADDR.load(deps.as_mut().storage, "1").unwrap()
        );
    }

    #[test]
    fn test_execute_register_module_validate() {
        let mut deps = mock_dependencies(&[]);

        let module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("address".to_string()),
        };
        let deps_mut = deps.as_mut();
        CONTRACT_OWNER
            .save(deps_mut.storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_register_module(
            &deps_mut.querier,
            deps_mut.storage,
            deps_mut.api,
            "owner",
            &module,
            ADOType::CW20,
            true,
        );

        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err(),
        );

        let res = execute_register_module(
            &deps_mut.querier,
            deps_mut.storage,
            deps_mut.api,
            "owner",
            &module,
            ADOType::CW20,
            false,
        )
        .unwrap();

        assert_eq!(
            Response::default().add_attribute("action", "register_module"),
            res
        );
    }

    #[test]
    fn test_execute_alter_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("sender", &[]);
        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
        };
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20);

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_alter_module_addr() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
        };

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        MODULE_INFO
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();
        MODULE_ADDR
            .save(deps.as_mut().storage, "1", &Addr::unchecked("address"))
            .unwrap();

        let module = Module {
            module_type: ModuleType::Receipt,
            instantiate: InstantiateType::Address("other_address".to_string()),
        };

        let res =
            execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20).unwrap();

        assert_eq!(
            Response::default()
                .add_attribute("action", "alter_module")
                .add_attribute("module_idx", "1"),
            res
        );

        assert_eq!(
            module,
            MODULE_INFO.load(deps.as_mut().storage, "1").unwrap()
        );

        assert_eq!(
            "other_address".to_string(),
            MODULE_ADDR.load(deps.as_mut().storage, "1").unwrap()
        );
    }

    #[test]
    fn test_execute_alter_module_nonexisting_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("address".to_string()),
        };

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20);

        assert_eq!(ContractError::ModuleDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_alter_module_incompatible_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: ModuleType::Auction,
            instantiate: InstantiateType::Address("address".to_string()),
        };

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        MODULE_INFO
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();
        MODULE_ADDR
            .save(deps.as_mut().storage, "1", &Addr::unchecked("address"))
            .unwrap();

        let res = execute_alter_module(deps.as_mut(), info, 1u64.into(), &module, ADOType::CW20);

        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err(),
        );
    }

    #[test]
    fn test_execute_deregister_module_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("sender", &[]);
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_deregister_module(deps.as_mut(), info, 1u64.into());

        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_deregister_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let module = Module {
            module_type: ModuleType::AddressList,
            instantiate: InstantiateType::Address("address".to_string()),
        };

        MODULE_INFO
            .save(deps.as_mut().storage, "1", &module)
            .unwrap();

        MODULE_ADDR
            .save(deps.as_mut().storage, "1", &Addr::unchecked("address"))
            .unwrap();

        let res = execute_deregister_module(deps.as_mut(), info, 1u64.into()).unwrap();

        assert_eq!(
            Response::default()
                .add_attribute("action", "deregister_module")
                .add_attribute("module_idx", "1"),
            res
        );

        assert!(!MODULE_ADDR.has(deps.as_mut().storage, "1"));
        assert!(!MODULE_INFO.has(deps.as_mut().storage, "1"));
    }

    #[test]
    fn test_execute_deregister_module_nonexisting_module() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("owner", &[]);
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();

        let res = execute_deregister_module(deps.as_mut(), info, 1u64.into());

        assert_eq!(ContractError::ModuleDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn test_process_module_response() {
        let res: Option<Response> = process_module_response(Ok(Response::new())).unwrap();
        assert_eq!(Some(Response::new()), res);

        let res: Option<Response> = process_module_response(Err(StdError::generic_err(
            "XXXXXXX UnsupportedOperation XXXXXXX",
        )))
        .unwrap();
        assert_eq!(None, res);

        let res: ContractError =
            process_module_response::<Response>(Err(StdError::generic_err("AnotherError")))
                .unwrap_err();
        assert_eq!(
            ContractError::Std(StdError::generic_err("AnotherError")),
            res
        );
    }
}
