use crate::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::ensure;

pub const RATES: &str = "rates";
pub const BIDS: &str = "bids";
pub const ADDRESS_LIST: &str = "address_list";
pub const AUCTION: &str = "auction";
pub const RECEIPT: &str = "receipt";
pub const OTHER: &str = "other";

/// A struct describing a token module, provided with the instantiation message this struct is used to record the info about the module and how/if it should be instantiated
#[cw_serde]
pub struct Module {
    pub module_name: Option<String>,
    pub address: String,
    pub is_mutable: bool,
}

impl Module {
    pub fn new(
        module_name: impl Into<String>,
        address: impl Into<String>,
        is_mutable: bool,
    ) -> Module {
        Module {
            module_name: Some(module_name.into()),
            address: address.into(),
            is_mutable,
        }
    }

    /// Validates `self` by checking that it is unique, does not conflict with any other module,
    /// and does not conflict with the creating ADO.
    pub fn validate(&self, modules: &[Module], ado_type: &str) -> Result<(), ContractError> {
        // We allow multiple rates modules.
        if self.module_name != Some(RATES.to_string()) {
            ensure!(self.is_unique(modules), ContractError::ModuleNotUnique {});
        }

        if ado_type == "cw20" && contains_module(modules, AUCTION) {
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
            if self.module_name == m.module_name {
                total += 1;
            }
        });

        total == 1
    }
}

/// Checks if any element of `modules` contains one of type `module_name`.
fn contains_module(modules: &[Module], module_name: &str) -> bool {
    modules
        .iter()
        .any(|m| m.module_name == Some(module_name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_addresslist() {
        let addresslist_module = Module {
            module_name: Some(ADDRESS_LIST.to_owned()),
            address: "".to_string(),

            is_mutable: false,
        };

        let res = addresslist_module.validate(
            &[addresslist_module.clone(), addresslist_module.clone()],
            "cw721",
        );
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let auction_module = Module {
            module_name: Some(AUCTION.to_owned()),
            address: "".to_string(),
            is_mutable: false,
        };
        addresslist_module
            .validate(&[addresslist_module.clone(), auction_module], "cw721")
            .unwrap();
    }

    #[test]
    fn test_validate_auction() {
        let module = Module {
            module_name: Some(AUCTION.to_owned()),
            address: "".to_string(),
            is_mutable: false,
        };

        let res = module.validate(&[module.clone(), module.clone()], "cw721");
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let res = module.validate(&[module.clone()], "cw20");
        assert_eq!(
            ContractError::IncompatibleModules {
                msg: "An Auction module cannot be used for a CW20 ADO".to_string()
            },
            res.unwrap_err()
        );

        let other_module = Module {
            module_name: Some(RATES.to_owned()),
            address: "".to_string(),
            is_mutable: false,
        };
        module
            .validate(&[module.clone(), other_module], "cw721")
            .unwrap();
    }

    #[test]
    fn test_validate_rates() {
        let module = Module {
            module_name: Some(RATES.to_owned()),
            address: "".to_string(),
            is_mutable: false,
        };

        module
            .validate(&[module.clone(), module.clone()], "cw721")
            .unwrap();

        let other_module = Module {
            module_name: Some(ADDRESS_LIST.to_owned()),
            address: "".to_string(),
            is_mutable: false,
        };
        module
            .validate(&[module.clone(), other_module], "cw721")
            .unwrap();
    }

    #[test]
    fn test_validate_receipt() {
        let module = Module {
            module_name: Some(RECEIPT.to_owned()),
            address: "".to_string(),
            is_mutable: false,
        };

        let res = module.validate(&[module.clone(), module.clone()], "cw721");
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());

        let other_module = Module {
            module_name: Some(ADDRESS_LIST.to_owned()),
            address: "".to_string(),
            is_mutable: false,
        };
        module
            .validate(&[module.clone(), other_module], "cw721")
            .unwrap();
    }

    #[test]
    fn test_validate_uniqueness() {
        let module1 = Module {
            module_name: Some(RECEIPT.to_owned()),
            address: "addr1".to_string(),

            is_mutable: false,
        };

        let module2 = Module {
            module_name: Some(RECEIPT.to_owned()),
            address: "addr2".to_string(),

            is_mutable: false,
        };

        let res = module1.validate(&[module1.clone(), module2], "cw721");
        assert_eq!(ContractError::ModuleNotUnique {}, res.unwrap_err());
    }
}
