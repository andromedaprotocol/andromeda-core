use crate::{
    modules::{Module, ModuleDefinition},
    require::require,
};

use cosmwasm_std::{coin, BankMsg, Coin, StdError, StdResult, Uint128};

use super::Rate;

pub fn calculate_fee(fee_rate: Rate, payment: Coin) -> Coin {
    match fee_rate {
        Rate::Flat(rate) => coin(Uint128::from(rate.amount).u128(), rate.denom),
        Rate::Percent(rate) => {
            // [COM-03] Make sure that fee_rate between 0 and 100.
            require(
                // No need for rate >=0 due to type limits (Question: Should add or remove?)
                rate <= 100,
                 StdError::generic_err("Rate must be between 0 and 100%")
                ).unwrap();
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
// [COM-02] Changed parameter all_modules type from Vec to a reference of a slice. 
pub fn is_unique<M: Module>(module: &M, all_modules: &[ModuleDefinition]) -> bool {
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

pub fn deduct_funds(coins: &mut Vec<Coin>, funds: Coin) -> StdResult<bool> {
    let coin_amount = coins.iter_mut().find(|c| c.denom.eq(&funds.denom));

    match coin_amount {
        Some(mut c) => {
            require(
                c.amount >= funds.amount,
                StdError::generic_err("Not enough funds to deduct payment"),
            )?;
            c.amount = c.amount - funds.amount;
            Ok(true)
        }
        None => Err(StdError::generic_err("Not enough funds to deduct payment")),
    }
}

pub fn add_payment(payments: &mut Vec<BankMsg>, to: String, amount: Coin) {
    let payment = BankMsg::Send {
        to_address: to,
        amount: vec![amount],
    };

    payments.push(payment);
}

pub fn deduct_payment(payments: &mut Vec<BankMsg>, to: String, amount: Coin) -> StdResult<bool> {
    let payment = payments.iter_mut().find(|m| match m {
        BankMsg::Send { to_address, .. } => to_address.clone().eq(&to),
        _ => false,
    });

    match payment {
        Some(p) => {
            match p {
                BankMsg::Send { amount: am, .. } => {
                    deduct_funds(am, amount)?;
                }
                _ => {}
            }
            Ok(true)
        }
        None => Err(StdError::generic_err(
            "Not enough funds to deduct required payment!",
        )),
    }
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
    fn test_deduct_funds() {
        let mut funds: Vec<Coin> = vec![coin(100, "uluna")];

        deduct_funds(&mut funds, coin(10, "uluna")).unwrap();

        assert_eq!(Uint128::from(90 as u64), funds[0].amount);
        assert_eq!(String::from("uluna"), funds[0].denom);

        let mut funds: Vec<Coin> = vec![Coin {
            denom: String::from("uluna"),
            amount: Uint128::from(5 as u64),
        }];

        let e = deduct_funds(&mut funds, coin(10, "uluna")).unwrap_err();

        assert_eq!(
            StdError::generic_err("Not enough funds to deduct payment"),
            e
        );
    }

    #[test]
    fn test_add_payment() {
        let mut payments: Vec<BankMsg> = vec![];

        let _from = String::from("from");
        let to = String::from("to");
        let amount = coin(1, "uluna");

        let expected_payment = BankMsg::Send {
            to_address: to.clone(),
            amount: vec![amount.clone()],
        };

        add_payment(&mut payments, to, amount);

        assert_eq!(1, payments.len());
        assert_eq!(expected_payment, payments[0]);
    }

    #[test]
    fn deduct_payment_test() {
        let to = String::from("to");

        let mut payments: Vec<BankMsg> = vec![BankMsg::Send {
            to_address: to.clone(),
            amount: vec![Coin {
                amount: Uint128::from(100 as u64),
                denom: String::from("uluna"),
            }],
        }];

        let expected_payment = BankMsg::Send {
            to_address: to.clone(),
            amount: vec![Coin {
                amount: Uint128::from(90 as u64),
                denom: String::from("uluna"),
            }],
        };

        deduct_payment(&mut payments, to, coin(10, "uluna")).unwrap();

        assert_eq!(expected_payment, payments[0]);
    }

    #[test]
    fn test_calculate_fee() {
        let payment = coin(101, "uluna");
        let expected = coin(5, "uluna");
        let fee = Rate::Percent(4);

        let received = calculate_fee(fee, payment);

        assert_eq!(expected, received);

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
