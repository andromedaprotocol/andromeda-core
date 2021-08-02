use cosmwasm_std::{BankMsg, Coin, Env, StdError, StdResult, Storage, String, Uint128};

use crate::{
    hooks::payments::{add_payment, deduct_payment},
    token::collection::collection_read,
    token::exchangeable::Exchangeable,
};

pub fn required_payment<S: Storage>(
    storage: &S,
    _env: &Env,
    collection_symbol: &String,
    token_id: &i64,
    receivers: &Vec<String>,
    royalty_fee: i64,
) -> StdResult<Vec<Coin>> {
    let collection = collection_read(storage).load(collection_symbol.as_bytes())?;
    match collection.get_transfer_agreement(storage, token_id) {
        Ok(a) => {
            let amount = a.amount;
            let denom = a.denom;
            let tax_amount: Uint128 =
                amount.multiply_ratio(Uint128(royalty_fee as u128), 100 as u128);

            let fee = Coin {
                amount: tax_amount,
                denom: String::from(&denom),
            };

            let mut coins: Vec<Coin> = vec![];
            for _ in receivers {
                coins.push(fee.clone());
            }

            Ok(coins)
        }
        Err(e) => match e {
            StdError::NotFound { .. } => Ok(vec![]),
            _ => Err(e),
        },
    }
}

pub fn post_transfer_payments<S: Storage>(
    storage: &S,
    env: &Env,
    collection_symbol: &String,
    token_id: &i64,
    receivers: &Vec<String>,
    royalty_fee: i64,
    payments: &mut Vec<BankMsg>,
) -> StdResult<bool> {
    let collection = collection_read(storage).load(collection_symbol.as_bytes())?;
    let owner = collection.get_owner(storage, token_id)?;
    let required_payments = required_payment(
        storage,
        env,
        collection_symbol,
        token_id,
        receivers,
        royalty_fee,
    )?;

    for i in 0..receivers.to_vec().len() {
        let receiver = receivers[i].clone();
        let amount = required_payments[i].clone();

        deduct_payment(
            payments,
            env.contract.address.clone(),
            owner.clone(),
            amount.clone(),
        )?;
        add_payment(
            payments,
            env.contract.address.clone(),
            receiver,
            amount.clone(),
        )?;
    }
    Ok(true)
}

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::coins;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Api, Extern, Querier,
    };

    use crate::{
        contract::{handle, init},
        extensions::extension::Extension,
        msg::{HandleMsg, InitMsg},
    };

    fn create_token<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        sender: &str,
        name: String,
        symbol: String,
        extensions: Vec<Extension>,
    ) {
        let auth_env = mock_env(sender, &coins(2, "token"));
        let msg = HandleMsg::Create {
            name,
            symbol,
            extensions,
        };
        let _res = handle(deps, auth_env, msg).unwrap();
    }

    fn mint_token<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        owner: &str,
        token_id: i64,
    ) {
        let auth_env = mock_env(owner, &coins(2, "token"));
        let msg = HandleMsg::Mint {
            collection_symbol: String::from("TT"),
            token_id: token_id,
        };
        let _res = handle(deps, auth_env, msg).unwrap();
    }

    #[test]
    fn required_payments() {
        let mut deps = mock_dependencies(20, &coins(100, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        create_token(
            &mut deps,
            "creator",
            String::from("Test Token"),
            String::from("TT"),
            vec![],
        );
        mint_token(&mut deps, "creator", 1);

        let transfer_env = mock_env("creator", &coins(2, "token"));
        let msg = HandleMsg::CreateTransferAgreement {
            collection_symbol: String::from("TT"),
            token_id: 1,
            amount: Uint128(100),
            denom: String::from("uluna"),
            purchaser: String::from("purchaser"),
        };
        let _res = handle(&mut deps, transfer_env, msg).unwrap();

        let buyer_env = mock_env("purchaser", &coins(100, "uluna"));
        let payments = required_payment(
            &deps.storage,
            &buyer_env,
            &String::from("TT"),
            &1,
            &vec![String::from("recv")],
            2,
        )
        .unwrap();

        assert!(payments.len() > 0);
        assert_eq!(String::from("uluna"), payments[0].denom);
        assert_eq!(Uint128(2), payments[0].amount);
    }

    #[test]
    fn transfer_payments() {
        let mut deps = mock_dependencies(20, &coins(100, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        create_token(
            &mut deps,
            "creator",
            String::from("Test Token"),
            String::from("TT"),
            vec![],
        );
        mint_token(&mut deps, "creator", 1);

        let transfer_env = mock_env("creator", &coins(2, "token"));
        let msg = HandleMsg::CreateTransferAgreement {
            collection_symbol: String::from("TT"),
            token_id: 1,
            amount: Uint128(100),
            denom: String::from("uluna"),
            purchaser: String::from("purchaser"),
        };
        let _res = handle(&mut deps, transfer_env, msg).unwrap();

        let buyer_env = mock_env("purchaser", &coins(100, "uluna"));
        let mut payments: Vec<BankMsg> = vec![BankMsg::Send {
            from_address: buyer_env.contract.address.clone(),
            to_address: String::from("creator"),
            amount: vec![Coin {
                denom: String::from("uluna"),
                amount: Uint128(100),
            }],
        }];

        post_transfer_payments(
            &deps.storage,
            &buyer_env,
            &String::from("TT"),
            &1,
            &vec![String::from("recv")],
            2,
            &mut payments,
        )
        .unwrap();

        assert!(payments.len() > 0);

        match &payments.to_vec()[0] {
            BankMsg::Send {
                to_address, amount, ..
            } => {
                assert!(to_address.eq(&String::from("creator")));
                assert!(amount.len() > 0);
                assert_eq!(Uint128(98), amount[0].amount);
                assert_eq!(String::from("uluna"), amount[0].denom);
            }
        }

        match &payments.to_vec()[1] {
            BankMsg::Send {
                to_address, amount, ..
            } => {
                assert!(to_address.eq(&String::from("recv")));
                assert!(amount.len() > 0);
                assert_eq!(Uint128(2), amount[0].amount);
                assert_eq!(String::from("uluna"), amount[0].denom);
            }
        }
    }
}
