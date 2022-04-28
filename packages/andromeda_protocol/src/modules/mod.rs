pub mod address_list;
pub mod common;
pub mod hooks;
pub mod receipt;
pub mod royalties;
pub mod taxable;

use crate::modules::{
    address_list::AddressListModule,
    hooks::{HookResponse, MessageHooks},
    receipt::ReceiptModule,
    royalties::Royalty,
    taxable::Taxable,
};
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, MessageInfo, StdResult, Storage, Uint128};
use cw721::Expiration;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MODULES: Item<Modules> = Item::new("modules");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
/// A struct used to define a flat rate fee
pub struct FlatRate {
    /// The fee amount
    pub amount: Uint128,
    /// The fee denomination
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
/// An enum used to define various types of fees
pub enum Rate {
    /// A flat rate fee
    Flat(FlatRate),
    /// A percentage fee (integer)
    Percent(u64),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
/// Definitions for each module, used in the `InstantiateMsg` for the token contract to define any modules assigned to the contract
pub enum ModuleDefinition {
    /// A whitelist module
    Whitelist {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract moderators. Used in combination with a valid `code_id` parameter
        moderators: Option<Vec<String>>,
    },
    /// A blacklist module
    Blacklist {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract moderators. Used in combination with a valid `code_id` parameter
        moderators: Option<Vec<String>>,
    },
    /// A tax module. Required payments are paid by the purchaser.
    Taxable {
        /// The tax rate
        rate: Rate,
        /// The receiving addresses of the fee
        receivers: Vec<String>,
        /// An optional description of the fee
        description: Option<String>,
    },
    /// A royalty module. Required payments are paid by the seller.
    Royalties {
        /// The royalty rate
        rate: Rate,
        /// The receiving addresses of the fee
        receivers: Vec<String>,
        /// An optional description of the fee
        description: Option<String>,
    },
    /// A receipt module
    Receipt {
        /// The address of the module contract
        address: Option<String>,
        /// A valid code ID for the module contract. Used upon contract instantiation to instantiate a new module contract.
        code_id: Option<u64>,
        /// A vector of contract moderators. Used in combination with a valid `code_id` parameter
        moderators: Option<Vec<String>>,
    },
}

pub trait Module: MessageHooks {
    fn validate(&self, modules: Vec<ModuleDefinition>) -> StdResult<bool>;
    fn as_definition(&self) -> ModuleDefinition;
    fn get_contract_address(&self, _storage: &dyn Storage) -> Option<String> {
        None
    }
}

impl ModuleDefinition {
    pub fn name(&self) -> String {
        String::from(match self {
            ModuleDefinition::Receipt { .. } => "receipt",
            ModuleDefinition::Royalties { .. } => "royalty",
            ModuleDefinition::Whitelist { .. } => "whitelist",
            ModuleDefinition::Blacklist { .. } => "blacklist",
            ModuleDefinition::Taxable { .. } => "tax",
        })
    }
    pub fn as_module(&self) -> Box<dyn Module> {
        match self {
            ModuleDefinition::Whitelist {
                address,
                code_id,
                moderators,
            } => Box::from(AddressListModule {
                moderators: moderators.clone(),
                address: address.clone(),
                // [MOD-01] Dereferencing the borrows and removing clone for u64.
                code_id: *code_id,
                inclusive: true,
            }),
            ModuleDefinition::Blacklist {
                address,
                code_id,
                moderators,
            } => Box::from(AddressListModule {
                moderators: moderators.clone(),
                address: address.clone(),
                code_id: *code_id,
                inclusive: false,
            }),
            ModuleDefinition::Taxable {
                rate,
                receivers,
                description,
            } => Box::from(Taxable {
                rate: rate.clone(),
                receivers: receivers.clone(),
                description: description.clone(),
            }),
            ModuleDefinition::Royalties {
                rate,
                receivers,
                description,
            } => Box::from(Royalty {
                rate: rate.clone(),
                receivers: receivers.to_vec(),
                description: description.clone(),
            }),
            ModuleDefinition::Receipt {
                moderators,
                address,
                code_id,
            } => Box::from(ReceiptModule {
                moderators: moderators.clone(),
                address: address.clone(),
                code_id: *code_id,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
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
            .to_vec()
            .into_iter()
            .map(|d| d.as_module())
            .collect()
    }
    pub fn validate(&self) -> StdResult<bool> {
        for module in self.to_modules() {
            module.validate(self.module_defs.clone())?;
        }

        Ok(true)
    }
    pub fn on_instantiate(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_instantiate(deps, info.clone(), env.clone())?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_execute(&self, deps: &DepsMut, info: MessageInfo, env: Env) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_execute(&deps, info.clone(), env.clone())?;
        }

        Ok(())
    }
    pub fn on_mint(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        token_id: String,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_mint(&deps, info.clone(), env.clone(), token_id.clone())?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_transfer(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        recipient: String,
        token_id: String,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_transfer(
                &deps,
                info.clone(),
                env.clone(),
                recipient.clone(),
                token_id.clone(),
            )?;
            resp = resp.add_resp(mod_res)
        }

        Ok(resp)
    }
    pub fn on_transfer_agreement(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        token_id: String,
        purchaser: String,
        amount: u128,
        denom: String,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_transfer_agreement(
                &deps,
                info.clone(),
                env.clone(),
                token_id.clone(),
                purchaser.clone(),
                amount,
                denom.clone(),
            )?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_agreed_transfer(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        payments: &mut Vec<BankMsg>,
        owner: String,
        purchaser: String,
        amount: Coin,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_resp = module.on_agreed_transfer(
                &deps,
                info.clone(),
                env.clone(),
                payments,
                owner.clone(),
                purchaser.clone(),
                amount.clone(),
            )?;

            resp = resp.add_resp(mod_resp);
        }

        Ok(resp)
    }
    pub fn on_send(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        contract: String,
        token_id: String,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_send(
                &deps,
                info.clone(),
                env.clone(),
                contract.clone(),
                token_id.clone(),
            )?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_approve(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        sender: String,
        token_id: String,
        expires: Option<Expiration>,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_approve(
                &deps,
                info.clone(),
                env.clone(),
                sender.clone(),
                token_id.clone(),
                expires.clone(),
            )?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_revoke(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        sender: String,
        token_id: String,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_revoke(
                &deps,
                info.clone(),
                env.clone(),
                sender.clone(),
                token_id.clone(),
            )?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_approve_all(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        operator: String,
        expires: Option<Expiration>,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_approve_all(
                &deps,
                info.clone(),
                env.clone(),
                operator.clone(),
                expires.clone(),
            )?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_revoke_all(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        operator: String,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res =
                module.on_revoke_all(&deps, info.clone(), env.clone(), operator.clone())?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_burn(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        token_id: String,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_burn(&deps, info.clone(), env.clone(), token_id.clone())?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
    pub fn on_archive(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        token_id: String,
    ) -> StdResult<HookResponse> {
        let modules = self.to_modules();
        let mut resp = HookResponse::default();
        for module in modules {
            let mod_res = module.on_burn(&deps, info.clone(), env.clone(), token_id.clone())?;
            resp = resp.add_resp(mod_res);
        }

        Ok(resp)
    }
}

pub fn store_modules(storage: &mut dyn Storage, modules: Modules) -> StdResult<()> {
    //Validate each module before storing
    modules.validate()?;

    MODULES.save(storage, &modules)
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
) -> StdResult<HookResponse> {
    let mut resp = HookResponse::default();

    for module_opt in modules {
        match module_opt {
            Some(module) => {
                //On instantiate generates instantiation message for a module (if it is required)
                let hook_resp = module.on_instantiate(&deps, info.clone(), env.clone())?;
                resp = resp.add_resp(hook_resp);
            }
            None => {}
        }
    }

    Ok(resp)
}
