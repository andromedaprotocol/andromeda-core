use crate::modules::{Module, ModuleDefinition};

use cosmwasm_std::{StdError, StdResult};

pub fn require(precond: bool, err: StdError) -> StdResult<bool> {
    match precond {
        true => Ok(true),
        false => Err(err),
    }
}

pub fn is_unique<M: Module>(module: &M, all_modules: &Vec<ModuleDefinition>) -> bool {
    let definition = module.as_definition();
    let mut total = 0;

    all_modules.into_iter().for_each(|d| {
        if std::mem::discriminant(d) == std::mem::discriminant(&definition) {
            total += 1;
        } else {
            total += 0;
        }
    });

    total == 1
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::HumanAddr;

    use crate::whitelist::Whitelist;

    use super::*;

    #[test]
    fn test_is_unique() {
        let module = Whitelist { moderators: vec![] };
        let duplicate_module = ModuleDefinition::WhiteList { moderators: vec![] };
        let similar_module = ModuleDefinition::WhiteList {
            moderators: vec![HumanAddr::default()],
        };
        let other_module = ModuleDefinition::Taxable {
            tax: 2,
            receivers: vec![],
        };

        let valid = vec![module.as_definition().clone(), other_module.clone()];
        assert_eq!(is_unique(&module, &valid), true);

        let duplicate = vec![
            module.as_definition().clone(),
            other_module.clone(),
            duplicate_module,
        ];

        assert_eq!(is_unique(&module, &duplicate), false);

        let similar = vec![module.as_definition().clone(), similar_module];
        assert_eq!(is_unique(&module, &similar), false);
    }
}
