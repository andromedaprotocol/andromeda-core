pub mod address_list;
pub mod common;
pub mod hooks;
pub mod receipt;

use ::common::{ado_base::query_get, encode_binary, error::ContractError, require};

use crate::{
    modules::{
        address_list::AddressListModule,
        hooks::{HookResponse, MessageHooks},
        receipt::ReceiptModule,
    },
    primitive::{GetValueResponse, Primitive},
};
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, QuerierWrapper, StdResult, Storage, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MODULES: Item<Modules> = Item::new("modules");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ADORate {
    /// The address of the primitive contract.
    pub address: String,
    /// The key of the primitive in the primitive contract.
    pub key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// An enum used to define various types of fees
pub enum Rate {
    /// A flat rate fee
    Flat(Coin),
    /// A percentage fee (integer)
    Percent(Uint128),
    External(ADORate),
}

impl Rate {
    /// Validates that a given rate is non-zero. It is expected that the Rate is not an
    /// External Rate.
    pub fn is_non_zero(&self) -> Result<bool, ContractError> {
        match self {
            Rate::Flat(coin) => Ok(coin.amount > Uint128::zero()),
            Rate::Percent(rate) => Ok(rate > &Uint128::zero()),
            Rate::External(_) => Err(ContractError::UnexpectedExternalRate {}),
        }
    }

    /// Validates `self` and returns an "unwrapped" version of itself wherein if it is an External
    /// Rate, the actual rate value is retrieved from the Primitive Contract.
    pub fn validate(&self, querier: &QuerierWrapper) -> Result<Rate, ContractError> {
        let rate = self.clone().get_rate(querier)?;
        require(rate.is_non_zero()?, ContractError::InvalidRate {})?;

        if let Rate::Percent(rate) = rate {
            require(
                rate <= Uint128::from(100u128),
                ContractError::InvalidRate {},
            )?;
        }

        Ok(rate)
    }

    /// If `self` is Flat or Percent it returns itself. Otherwise it queries the primitive contract
    /// and retrieves the actual Flat or Percent rate.
    fn get_rate(self, querier: &QuerierWrapper) -> Result<Rate, ContractError> {
        match self {
            Rate::Flat(_) => Ok(self),
            Rate::Percent(_) => Ok(self),
            Rate::External(ado_rate) => {
                let response: GetValueResponse = query_get(
                    Some(encode_binary(&ado_rate.key)?),
                    ado_rate.address,
                    querier,
                )?;
                match response.value {
                    Primitive::Coin(coin) => Ok(Rate::Flat(coin)),
                    Primitive::Uint128(value) => Ok(Rate::Percent(value)),
                    _ => Err(ContractError::ParsingError {
                        err: "Stored rate is not a coin or Uint128".to_string(),
                    }),
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// Definitions for each module, used in the `InstantiateMsg` for the token contract to define any modules assigned to the contract
pub enum ModuleDefinition {
    /// A whitelist module
    Whitelist {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract operators. Used in combination with a valid `code_id` parameter
        operators: Option<Vec<String>>,
    },
    /// A blacklist module
    Blacklist {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract operators. Used in combination with a valid `code_id` parameter
        operators: Option<Vec<String>>,
    },
    /// A receipt module
    Receipt {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract operators. Used in combination with a valid `code_id` parameter
        operators: Option<Vec<String>>,
    },
}

pub trait Module: MessageHooks {
    fn validate(
        &self,
        modules: Vec<ModuleDefinition>,
        querier: &QuerierWrapper,
    ) -> Result<bool, ContractError>;
    fn as_definition(&self) -> ModuleDefinition;
    fn get_contract_address(&self, _storage: &dyn Storage) -> Option<String> {
        None
    }
}

impl ModuleDefinition {
    pub fn name(&self) -> String {
        String::from(match self {
            ModuleDefinition::Receipt { .. } => "receipt",
            ModuleDefinition::Whitelist { .. } => "whitelist",
            ModuleDefinition::Blacklist { .. } => "blacklist",
        })
    }
    pub fn as_module(&self) -> Box<dyn Module> {
        match self {
            ModuleDefinition::Whitelist {
                address,
                code_id,
                operators,
            } => Box::from(AddressListModule {
                operators: operators.clone(),
                address: address.clone(),
                // [MOD-01] Dereferencing the borrows and removing clone for u64.
                code_id: *code_id,
                inclusive: true,
            }),
            ModuleDefinition::Blacklist {
                address,
                code_id,
                operators,
            } => Box::from(AddressListModule {
                operators: operators.clone(),
                address: address.clone(),
                code_id: *code_id,
                inclusive: false,
            }),
            ModuleDefinition::Receipt {
                operators,
                address,
                code_id,
            } => Box::from(ReceiptModule {
                operators: operators.clone(),
                address: address.clone(),
                code_id: *code_id,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// Helping struct to aid in hook execution.
/// The `Modules` struct implements all hooks that a `Module` may implement.
pub struct Modules {
    pub module_defs: Vec<ModuleDefinition>,
}

impl Modules {
    pub fn new(module_defs: Vec<ModuleDefinition>) -> Modules {
        Modules { module_defs }
    }
    pub fn default() -> Modules {
        Modules {
            module_defs: vec![],
        }
    }
    pub fn to_modules(&self) -> Vec<Box<dyn Module>> {
        self.module_defs
            .iter()
            .cloned()
            .map(|d| d.as_module())
            .collect()
    }
    pub fn validate(&self, querier: &QuerierWrapper) -> Result<bool, ContractError> {
        for module in self.to_modules() {
            module.validate(self.module_defs.clone(), querier)?;
        }

        Ok(true)
    }
    pub fn hook<F>(&self, f: F) -> Result<HookResponse, ContractError>
    where
        F: Fn(Box<dyn Module>) -> Result<HookResponse, ContractError>,
    {
        let modules = self.to_modules();
        let mut res = HookResponse::default();
        for module in modules {
            res = res.add_resp(f(module)?);
        }

        Ok(res)
    }
}

pub fn store_modules(
    storage: &mut dyn Storage,
    modules: Modules,
    querier: &QuerierWrapper,
) -> Result<(), ContractError> {
    //Validate each module before storing
    modules.validate(querier)?;

    Ok(MODULES.save(storage, &modules)?)
}

pub fn read_modules(storage: &dyn Storage) -> StdResult<Modules> {
    let module_defs = MODULES.may_load(storage).unwrap_or_default();

    match module_defs {
        Some(mods) => Ok(mods),
        None => Ok(Modules::default()),
    }
}

/// Generates instantiation messgaes for a list of modules
///
/// Returns a HookResponse object containing the instantiation messages
pub fn generate_instantiate_msgs(
    deps: &DepsMut,
    info: MessageInfo,
    env: Env,
    modules: Vec<Option<impl Module>>,
) -> Result<HookResponse, ContractError> {
    let mut resp = HookResponse::default();

    for module in modules.into_iter().flatten() {
        //On instantiate generates instantiation message for a module (if it is required)
        let hook_resp = module.on_instantiate(deps, info.clone(), env.clone())?;
        resp = resp.add_resp(hook_resp);
    }

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_PRIMITIVE_CONTRACT};
    use cosmwasm_std::coin;

    use super::*;

    #[test]
    fn test_validate_external_rate() {
        let mut deps = mock_dependencies_custom(&[]);

        let rate = Rate::External(ADORate {
            address: MOCK_PRIMITIVE_CONTRACT.to_string(),
            key: Some("percent".to_string()),
        });
        let validated_rate = rate.validate(&deps.as_mut().querier).unwrap();
        let expected_rate = Rate::Percent(1u128.into());
        assert_eq!(expected_rate, validated_rate);

        let rate = Rate::External(ADORate {
            address: MOCK_PRIMITIVE_CONTRACT.to_string(),
            key: Some("flat".to_string()),
        });
        let validated_rate = rate.validate(&deps.as_mut().querier).unwrap();
        let expected_rate = Rate::Flat(coin(1u128, "uusd"));
        assert_eq!(expected_rate, validated_rate);
    }
}
