pub mod blacklist;
pub mod common;
pub mod hooks;
pub mod royalties;
pub mod taxable;
pub mod whitelist;

use crate::modules::taxable::Taxable;
use crate::modules::{hooks::MessageHooks, whitelist::Whitelist};
use crate::token::ExecuteMsg;
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, MessageInfo, StdResult, Storage};
use cw721::Expiration;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use self::blacklist::Blacklist;
use self::royalties::Royalty;

// const KEY_MODULES: &[u8] = b"modules";
pub const MODULES: Item<Modules> = Item::new("modules");

pub type Fee = u128;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleDefinition {
    Whitelist {
        moderators: Vec<String>,
    },
    Blacklist {
        moderators: Vec<String>,
    },
    Taxable {
        tax: Fee,
        receivers: Vec<String>,
    },
    Royalties {
        fee: Fee,
        receivers: Vec<String>,
        description: Option<String>,
    },
}

pub trait Module: MessageHooks {
    fn validate(&self, extensions: Vec<ModuleDefinition>) -> StdResult<bool>;
    fn as_definition(&self) -> ModuleDefinition;
}

impl ModuleDefinition {
    pub fn as_module(&self) -> Box<dyn Module> {
        match self {
            ModuleDefinition::Whitelist { moderators } => Box::from(Whitelist {
                moderators: moderators.clone(),
            }),
            ModuleDefinition::Blacklist { moderators } => Box::from(Blacklist {
                moderators: moderators.clone(),
            }),
            ModuleDefinition::Taxable { tax, receivers } => Box::from(Taxable {
                tax: tax.clone(),
                receivers: receivers.clone(),
            }),
            ModuleDefinition::Royalties {
                fee,
                receivers,
                description,
            } => Box::from(Royalty {
                fee: fee.clone(),
                receivers: receivers.to_vec(),
                description: description.clone(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
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
    pub fn on_execute(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        msg: ExecuteMsg,
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_execute(&deps, info.clone(), env.clone(), msg.clone())?;
        }

        Ok(())
    }
    pub fn on_mint(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        token_id: String,
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_mint(&deps, info.clone(), env.clone(), token_id.clone())?;
        }

        Ok(())
    }
    pub fn on_transfer(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        recipient: String,
        token_id: String,
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_transfer(
                &deps,
                info.clone(),
                env.clone(),
                recipient.clone(),
                token_id.clone(),
            )?;
        }

        Ok(())
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
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_transfer_agreement(
                &deps,
                info.clone(),
                env.clone(),
                token_id.clone(),
                purchaser.clone(),
                amount.clone(),
                denom.clone(),
            )?;
        }

        Ok(())
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
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_agreed_transfer(
                &deps,
                info.clone(),
                env.clone(),
                payments,
                owner.clone(),
                purchaser.clone(),
                amount.clone(),
            )?;
        }

        Ok(())
    }
    pub fn on_send(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        contract: String,
        token_id: String,
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_send(
                &deps,
                info.clone(),
                env.clone(),
                contract.clone(),
                token_id.clone(),
            )?;
        }

        Ok(())
    }
    pub fn on_approve(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        sender: String,
        token_id: String,
        expires: Option<Expiration>,
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_approve(
                &deps,
                info.clone(),
                env.clone(),
                sender.clone(),
                token_id.clone(),
                expires.clone(),
            )?;
        }

        Ok(())
    }
    pub fn on_revoke(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        sender: String,
        token_id: String,
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_revoke(
                &deps,
                info.clone(),
                env.clone(),
                sender.clone(),
                token_id.clone(),
            )?;
        }

        Ok(())
    }
    pub fn on_approve_all(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        operator: String,
        expires: Option<Expiration>,
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_approve_all(
                &deps,
                info.clone(),
                env.clone(),
                operator.clone(),
                expires.clone(),
            )?;
        }

        Ok(())
    }
    pub fn on_revoke_all(
        &self,
        deps: &DepsMut,
        info: MessageInfo,
        env: Env,
        operator: String,
    ) -> StdResult<()> {
        let modules = self.to_modules();
        for module in modules {
            module.on_revoke_all(&deps, info.clone(), env.clone(), operator.clone())?;
        }

        Ok(())
    }
}

//Converts a ModuleDefinition to a Module struct

pub fn store_modules(
    storage: &mut dyn Storage,
    module_defs: Vec<ModuleDefinition>,
) -> StdResult<()> {
    //Validate each module before storing
    let modules = Modules::new(module_defs);
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
