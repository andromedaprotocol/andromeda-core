use crate::extension::Extension;
use cosmwasm_std::{HumanAddr, StdError, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

static KEY_TAXABLE: &[u8] = b"taxable";

pub struct Taxable {
    fee: i64,
    receivers: Vec<HumanAddr>,
}

pub fn store_tax_config<S: Storage>(storage: &mut S, config: Extension) -> StdResult<()> {
    match config {
        Extension::TaxableExtension { .. } => {
            let updated_config = match read_tax_config(storage) {
                Some(curr_config) => {
                    let mut new_config = curr_config.clone();
                    new_config.push(config);
                    new_config
                }
                None => {
                    let new_config = vec![config];
                    new_config
                }
            };
            singleton(storage, KEY_TAXABLE).save(&updated_config)
        }
        _ => Err(StdError::generic_err(
            "Incorrect extension type, expected TaxableExtension.",
        )),
    }
}

pub fn read_tax_config<S: Storage>(storage: &S) -> Option<Vec<Extension>> {
    match singleton_read(storage, KEY_TAXABLE).load() {
        Ok(ext) => Some(ext),
        Err(_e) => None,
    }
}

// pub fn required_payment<S: Storage>(
//     storage: &S,
//     _env: &Env,
//     collection_symbol: &String,
//     token_id: &i64,
//     receivers: &Vec<HumanAddr>,
//     tax_fee: i64,
// ) -> StdResult<Vec<Coin>> {
//     let collection = collection_read(storage).load(collection_symbol.as_bytes())?;
//     match collection.get_transfer_agreement(storage, token_id) {
//         Ok(a) => {
//             let amount = a.amount;
//             let denom = a.denom;
//             let tax_amount: Uint128 = amount.multiply_ratio(Uint128(tax_fee as u128), 100 as u128);

//             let fee = Coin {
//                 amount: tax_amount,
//                 denom: String::from(&denom),
//             };

//             let mut coins: Vec<Coin> = vec![];
//             for _ in receivers {
//                 coins.push(fee.clone());
//             }

//             Ok(coins)
//         }
//         Err(e) => match e {
//             StdError::NotFound { .. } => Ok(vec![]),
//             _ => Err(e),
//         },
//     }
// }

// pub fn post_transfer_payments<S: Storage>(
//     storage: &S,
//     env: &Env,
//     collection_symbol: &String,
//     token_id: &i64,
//     receivers: &Vec<HumanAddr>,
//     tax_fee: i64,
//     payments: &mut Vec<BankMsg>,
// ) -> StdResult<bool> {
//     let required_payments = required_payment(
//         storage,
//         env,
//         collection_symbol,
//         token_id,
//         receivers,
//         tax_fee,
//     )?
//     .to_vec();

//     for i in 0..required_payments.len() {
//         let from_address = &env.contract.address.clone();
//         let payment = required_payments[i].clone();
//         let receiver = receivers[i].clone();

//         add_payment(payments, from_address.clone(), receiver, payment)?;
//     }

//     Ok(true)
// }

// #[cfg(test)]
// mod test {
//     use cosmwasm_std::coins;
//     use cosmwasm_std::{
//         testing::{mock_dependencies, mock_env},
//         Api, Extern, Querier,
//     };

//     use crate::{
//         contract::{handle, init},
//         extensions::extension::Extension,
//         msg::{HandleMsg, InitMsg},
//     };

//     use super::*;

//     fn create_token<S: Storage, A: Api, Q: Querier>(
//         deps: &mut Extern<S, A, Q>,
//         sender: &str,
//         name: String,
//         symbol: String,
//         extensions: Vec<Extension>,
//     ) {
//         let auth_env = mock_env(sender, &coins(2, "token"));
//         let msg = HandleMsg::Create {
//             name,
//             symbol,
//             extensions,
//         };
//         let _res = handle(deps, auth_env, msg).unwrap();
//     }

//     fn mint_token<S: Storage, A: Api, Q: Querier>(
//         deps: &mut Extern<S, A, Q>,
//         owner: &str,
//         token_id: i64,
//     ) {
//         let auth_env = mock_env(owner, &coins(2, "token"));
//         let msg = HandleMsg::Mint {
//             collection_symbol: String::from("TT"),
//             token_id: token_id,
//         };
//         let _res = handle(deps, auth_env, msg).unwrap();
//     }

//     #[test]
//     fn required_payments() {
//         let mut deps = mock_dependencies(20, &coins(100, "token"));

//         let msg = InitMsg {};
//         let env = mock_env("creator", &coins(2, "token"));
//         let _res = init(&mut deps, env, msg).unwrap();

//         create_token(
//             &mut deps,
//             "creator",
//             String::from("Test Token"),
//             String::from("TT"),
//             vec![],
//         );
//         mint_token(&mut deps, "creator", 1);

//         let transfer_env = mock_env("creator", &coins(2, "token"));
//         let msg = HandleMsg::CreateTransferAgreement {
//             collection_symbol: String::from("TT"),
//             token_id: 1,
//             amount: Uint128(100),
//             denom: String::from("uluna"),
//             purchaser: HumanAddr::from("purchaser"),
//         };
//         let _res = handle(&mut deps, transfer_env, msg).unwrap();

//         let buyer_env = mock_env("purchaser", &coins(100, "uluna"));
//         let payments = required_payment(
//             &deps.storage,
//             &buyer_env,
//             &String::from("TT"),
//             &1,
//             &vec![HumanAddr::from("recv")],
//             2,
//         )
//         .unwrap();

//         assert!(payments.len() > 0);
//         assert_eq!(String::from("uluna"), payments[0].denom);
//         assert_eq!(Uint128(2), payments[0].amount);
//     }

//     #[test]
//     fn transfer_payments() {
//         let mut deps = mock_dependencies(20, &coins(100, "token"));

//         let msg = InitMsg {};
//         let env = mock_env("creator", &coins(2, "token"));
//         let _res = init(&mut deps, env, msg).unwrap();

//         create_token(
//             &mut deps,
//             "creator",
//             String::from("Test Token"),
//             String::from("TT"),
//             vec![],
//         );
//         mint_token(&mut deps, "creator", 1);

//         let transfer_env = mock_env("creator", &coins(2, "token"));
//         let msg = HandleMsg::CreateTransferAgreement {
//             collection_symbol: String::from("TT"),
//             token_id: 1,
//             amount: Uint128(100),
//             denom: String::from("uluna"),
//             purchaser: HumanAddr::from("purchaser"),
//         };
//         let _res = handle(&mut deps, transfer_env, msg).unwrap();

//         let buyer_env = mock_env("purchaser", &coins(100, "uluna"));
//         let mut payments: Vec<BankMsg> = vec![];

//         post_transfer_payments(
//             &deps.storage,
//             &buyer_env,
//             &String::from("TT"),
//             &1,
//             &vec![HumanAddr::from("recv")],
//             2,
//             &mut payments,
//         )
//         .unwrap();

//         assert!(payments.len() > 0);

//         match &payments.to_vec()[0] {
//             BankMsg::Send {
//                 to_address, amount, ..
//             } => {
//                 assert!(to_address.eq(&HumanAddr::from("recv")));
//                 assert!(amount.len() > 0);
//                 assert_eq!(Uint128(2), amount[0].amount);
//                 assert_eq!(String::from("uluna"), amount[0].denom);
//             }
//         }
//     }
// }
