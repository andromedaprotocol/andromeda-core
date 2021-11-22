use crate::{
    modules::{hooks::HookResponse, Module},
    require::require,
};
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, MessageInfo, StdError, StdResult, Uint128};

//Redundant? Can maybe use `Modules` struct?
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
                let hook_resp = module.on_instantiate(&deps, info.clone(), env.clone())?;
                resp = resp.add_resp(hook_resp);
            }
            None => {}
        }
    }

    Ok(resp)
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

pub fn add_funds(coins: &mut Vec<Coin>, funds: Coin) {
    let coin_amount = coins.iter_mut().find(|c| c.denom.eq(&funds.denom));

    match coin_amount {
        Some(mut c) => {
            c.amount = c.amount + funds.amount;
        }
        None => {
            coins.push(funds);
        }
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

/**
 * Calculcates remaining funds from required payments
 */
pub fn calculate_remaining_funds(sent: Vec<Coin>, payments: Vec<Coin>) -> StdResult<Vec<Coin>> {
    let mut remaining = sent.clone();

    for payment in payments {
        deduct_funds(&mut remaining, payment)?;
    }
    remaining.retain(|coin| coin.amount > Uint128::zero());

    Ok(remaining)
}

/**
 * Extracts required payments from response messages
 */
pub fn calculcate_required_payments(msgs: Vec<BankMsg>) -> Vec<Coin> {
    let mut required_payments: Vec<Coin> = vec![];
    for msg in msgs {
        match msg {
            BankMsg::Send { amount, .. } => {
                for coin in amount {
                    add_funds(&mut required_payments, coin);
                }
            }
            _ => {}
        }
    }

    required_payments
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{coin, coins, Coin, Uint128};

    use super::*;

    #[test]
    fn test_calculate_remaining_funds() {
        let sent_funds = [coins(100, "uluna"), coins(100, "ucosm"), coins(50, "uusd")].concat();
        let payments = [coins(20, "ucosm"), coins(90, "uluna"), coins(50, "uusd")].concat();
        let expected = [coins(10, "uluna"), coins(80, "ucosm")].concat();

        let remaining_funds = calculate_remaining_funds(sent_funds, payments.clone()).unwrap();

        assert_eq!(remaining_funds, expected);

        let insufficient_funds = [coins(10, "uluna")].concat();

        let error = calculate_remaining_funds(insufficient_funds, payments).unwrap_err();
        let expected_error = StdError::generic_err("Not enough funds to deduct payment");

        assert_eq!(error, expected_error);
    }

    #[test]
    fn test_calculate_required_payments() {
        let payment_msgs = vec![
            BankMsg::Send {
                to_address: String::default(),
                amount: coins(100, "uluna"),
            },
            BankMsg::Send {
                to_address: String::default(),
                amount: coins(10, "ucosm"),
            },
            BankMsg::Send {
                to_address: String::default(),
                amount: coins(10, "uluna"),
            },
        ];

        let required_payments = calculcate_required_payments(payment_msgs);
        let expected = [coins(110, "uluna"), coins(10, "ucosm")].concat();
        assert_eq!(required_payments, expected)
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
