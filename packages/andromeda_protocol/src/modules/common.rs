use crate::{
    modules::{Module, ModuleDefinition, Rate},
    require,
};

use cosmwasm_std::{coin, BankMsg, Coin, StdError, StdResult, Uint128};

/// Calculates a fee amount given a `Rate` and payment amount.
///
/// ## Arguments
/// * `fee_rate` - The `Rate` of the fee to be paid
/// * `payment` - The amount used to calculate the fee
///
/// Returns the fee amount in a `Coin` struct.
pub fn calculate_fee(fee_rate: Rate, payment: Coin) -> Coin {
    if payment.amount > Uint128::MAX.checked_div(Uint128::from(100u128)).unwrap() {
        panic!("Payment amount exceeds maximum value")
    }

    match fee_rate {
        Rate::Flat(rate) => coin(Uint128::from(rate.amount).u128(), rate.denom),
        Rate::Percent(rate) => {
            // [COM-03] Make sure that fee_rate between 0 and 100.
            require(
                // No need for rate >=0 due to type limits (Question: Should add or remove?)
                rate <= 100,
                StdError::generic_err("Rate must be between 0 and 100%"),
            )
            .unwrap();
            let mut fee_amount = payment.amount.multiply_ratio(rate, 100_u128).u128();

            //Always round any remainder up and prioritise the fee receiver
            let reversed_fee = (fee_amount * 100) / Uint128::from(rate).u128();
            if payment.amount.u128() > reversed_fee {
                // [COM-1] Added checked add to fee_amount rather than direct increment
                let res = fee_amount.checked_add(1);
                let _res = match res {
                    None => panic!("Problem adding: Overflow in addition"),
                    _ => fee_amount = res.unwrap(),
                };
            }

            coin(fee_amount, payment.denom)
        }
    }
}

// [COM-02] Changed parameter all_modules type from Vec to a reference of a slice.
/// Determines if a `ModuleDefinition` is unique within the context of a vector of `ModuleDefinition`
///
/// ## Arguments
/// * `module` - The module to check for uniqueness
/// * `all_modules` - The vector of modules containing the provided module
///
/// Returns a `boolean` representing whether the module is unique or not
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

/// Deducts a given amount from a vector of `Coin` structs. Alters the given vector, does not return a new vector.
///
/// ## Arguments
/// * `coins` - The vector of `Coin` structs from which to deduct the given funds
/// * `funds` - The amount to deduct
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

/// Adds a new payment message to a vector of `BankMsg` structs. Alters the provided vector, does not return a new vector.
///
/// ## Arguments
/// * `payments` - The vector of `BankMsg` structs for which to attach the new `BankMsg`
/// * `to` - The recipient of the payment
/// * `amount` - The amount to be sent
pub fn add_payment(payments: &mut Vec<BankMsg>, to: String, amount: Coin) {
    let payment = BankMsg::Send {
        to_address: to,
        amount: vec![amount],
    };

    payments.push(payment);
}

/// Deducts a given amount from a vector of `BankMsg` structs. Alters the provided vector, does not return a new vector.
///
/// ## Arguments
/// * `payments` - The vector of `BankMsg` structs for which to deduct the amount
/// * `to` - The recipient of the payment
/// * `amount` - The amount to be deducted
///
/// Errors if there is no payment from which to deduct the funds
pub fn deduct_payment(payments: &mut Vec<BankMsg>, to: String, amount: Coin) -> StdResult<bool> {
    let payment = payments.iter_mut().find(|m| match m {
        BankMsg::Send { to_address, .. } => to_address.eq(&to),
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
        // [COM-05] Misleading error message since it should check whether there is pending deductions and not if it has enough funds.
        None => Err(StdError::generic_err(
            "No pending payments for the given address!",
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
            amount: Uint128::from(5_u128),
            denom: "uluna".to_string(),
        });

        let received = calculate_fee(fee, payment);

        assert_eq!(expected, received);
    }

    #[test]
    #[should_panic]
    fn test_calculate_fee_max() {
        let payment = coin(Uint128::MAX.u128(), "uluna");
        let fee = Rate::Percent(4);

        calculate_fee(fee, payment);
    }
}
