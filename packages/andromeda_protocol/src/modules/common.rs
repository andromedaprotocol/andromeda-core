use crate::modules::{Module, ModuleDefinition};

use cosmwasm_std::{BankMsg, Coin, StdError, StdResult};

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
        //Compares enum values of given definitions
        if std::mem::discriminant(d) == std::mem::discriminant(&definition) {
            total += 1;
        } else {
            total += 0;
        }
    });

    total == 1
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
    use crate::modules::whitelist::Whitelist;
    use cosmwasm_std::{coin, Uint128};

    #[test]
    fn test_is_unique() {
        let module = Whitelist { moderators: vec![] };
        let duplicate_module = ModuleDefinition::WhiteList { moderators: vec![] };
        let similar_module = ModuleDefinition::WhiteList {
            moderators: vec![String::default()],
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
}
