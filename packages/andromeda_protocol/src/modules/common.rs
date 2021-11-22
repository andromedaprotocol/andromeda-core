use crate::modules::{Module, ModuleDefinition};

use cosmwasm_std::{coin, Coin, Uint128};

use super::Rate;

pub fn calculate_fee(fee_rate: Rate, payment: Coin) -> Coin {
    match fee_rate {
        Rate::Flat(rate) => coin(Uint128::from(rate.amount).u128(), rate.denom),
        Rate::Percent(rate) => {
            let mut fee_amount = payment.amount.multiply_ratio(rate, 100 as u128).u128();

            //Always round any remainder up and prioritise the fee receiver
            let reversed_fee = (fee_amount * 100) / Uint128::from(rate).u128();
            if payment.amount.u128() > reversed_fee {
                fee_amount += 1
            }

            coin(fee_amount, payment.denom)
        }
    }
}

pub fn is_unique<M: Module>(module: &M, all_modules: &Vec<ModuleDefinition>) -> bool {
    let definition = module.as_definition();
    let mut total = 0;
    all_modules.into_iter().for_each(|d| {
        //Compares enum values of given definitions
        if std::mem::discriminant(d) == std::mem::discriminant(&definition) {
            total += 1;
        } else {
            total += 0;
        }
    });

    total <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::address_list::AddressListModule;
    use crate::modules::FlatRate;
    use crate::modules::Rate;
    use cosmwasm_std::{coin, Uint128};

    #[test]
    fn test_is_unique() {
        let module = AddressListModule {
            moderators: Some(vec![]),
            address: None,
            code_id: None,
            inclusive: true,
        };
        let duplicate_module = ModuleDefinition::Whitelist {
            moderators: Some(vec![]),
            address: None,
            code_id: None,
        };
        let similar_module = ModuleDefinition::Whitelist {
            moderators: Some(vec![String::default()]),
            address: None,
            code_id: None,
        };
        let other_module = ModuleDefinition::Taxable {
            rate: Rate::Percent(2),
            receivers: vec![],
            description: None,
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

    #[test]
    fn test_calculate_fee() {
        let payment = coin(101, "uluna");
        let expected = coin(5, "uluna");
        let fee = Rate::Percent(4);

        let received = calculate_fee(fee, payment);

        assert_eq!(expected, received);

        let payment = coin(125, "uluna");
        let expected = coin(5, "uluna");
        let fee = Rate::Flat(FlatRate {
            amount: Uint128::from(5 as u128),
            denom: "uluna".to_string(),
        });

        let received = calculate_fee(fee, payment);

        assert_eq!(expected, received);
    }
}
