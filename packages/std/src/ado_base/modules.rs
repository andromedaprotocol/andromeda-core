use crate::amp::addresses::AndrAddr;
#[cfg(feature = "modules")]
use crate::error::ContractError;

use cosmwasm_schema::cw_serde;
#[cfg(feature = "modules")]
use cosmwasm_std::ensure;

/// A struct describing a token module, provided with the instantiation message this struct is used to record the info about the module and how/if it should be instantiated
#[cw_serde]
pub struct Module {
    pub name: Option<String>,
    pub address: AndrAddr,
    pub is_mutable: bool,
}

#[cfg(feature = "modules")]
impl Module {
    pub fn new(name: impl Into<String>, address: impl Into<String>, is_mutable: bool) -> Module {
        Module {
            name: Some(name.into()),
            address: AndrAddr::from_string(address.into()),
            is_mutable,
        }
    }

    /// Validates `self` by checking that it is unique, does not conflict with any other module,
    /// and does not conflict with the creating ADO.
    pub fn validate(&self, modules: &[Module]) -> Result<(), ContractError> {
        ensure!(self.is_unique(modules), ContractError::ModuleNotUnique {});

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
            if self.name == m.name {
                total += 1;
            }
        });

        total == 1
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "modules")]
    use super::*;

    #[test]
    #[cfg(feature = "modules")]
    fn test_validate_uniqueness() {
        let module1 = Module::new("module", "addr1", false);
        let module2 = Module::new("module", "addr2", false);

        let res = module1.validate(&[module1.clone(), module2]);
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());
    }
}
