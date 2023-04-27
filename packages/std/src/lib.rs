pub mod ado_base;
pub mod ado_contract;
pub mod amp;
pub mod common;
pub mod error;
pub mod os;

pub use andromeda_macros::andr_exec;
pub use strum_macros::AsRefStr;

#[cfg(test)]
pub mod testing;

use crate::error::ContractError;
use ado_base::{AndromedaQuery, QueryMsg};
use cosmwasm_std::{
    ensure, from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, QuerierWrapper, QueryRequest,
    SubMsg, WasmQuery,
};
use cw20::Cw20Coin;

use serde::{de::DeserializeOwned, Serialize};
use std::collections::BTreeMap;

use cosmwasm_schema::cw_serde;
#[cw_serde]
pub enum OrderBy {
    Asc,
    Desc,
}

pub fn parse_struct<T>(val: &Binary) -> Result<T, ContractError>
where
    T: DeserializeOwned,
{
    let data_res = from_binary(val);
    match data_res {
        Ok(data) => Ok(data),
        Err(err) => Err(ContractError::ParsingError {
            err: err.to_string(),
        }),
    }
}

pub fn parse_message<T: DeserializeOwned>(data: &Option<Binary>) -> Result<T, ContractError> {
    let data = unwrap_or_err(data, ContractError::MissingRequiredMessageData {})?;
    parse_struct::<T>(data)
}

pub fn encode_binary<T>(val: &T) -> Result<Binary, ContractError>
where
    T: Serialize,
{
    match to_binary(val) {
        Ok(encoded_val) => Ok(encoded_val),
        Err(err) => Err(err.into()),
    }
}

pub fn unwrap_or_err<T>(val_opt: &Option<T>, err: ContractError) -> Result<&T, ContractError> {
    match val_opt {
        Some(val) => Ok(val),
        None => Err(err),
    }
}

pub fn query_primitive<T>(
    querier: QuerierWrapper,
    contract_address: String,
    key: Option<String>,
) -> Result<T, ContractError>
where
    T: DeserializeOwned,
{
    let message = QueryMsg::AndrQuery(AndromedaQuery::Get(Some(to_binary(&key)?)));
    let resp: T = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_address,
        msg: encode_binary(&message)?,
    }))?;
    Ok(resp)
}

#[cw_serde]
pub enum Funds {
    Native(Coin),
    Cw20(Cw20Coin),
}

impl Funds {
    // There is probably a more idiomatic way of doing this with From and Into...
    pub fn try_get_coin(&self) -> Result<Coin, ContractError> {
        match self {
            Funds::Native(coin) => Ok(coin.clone()),
            Funds::Cw20(_) => Err(ContractError::ParsingError {
                err: "Funds is not of type Native".to_string(),
            }),
        }
    }
}

/// Merges bank messages to the same recipient to a single bank message. Any sub messages
/// that do not contain bank messages are left as is. Note: Original order is not necessarily maintained.
///
/// ## Arguments
/// * `msgs`  - The sub messages to merge.
///
/// Returns a Vec<SubMsg> containing the merged bank messages.
pub fn merge_sub_msgs(msgs: Vec<SubMsg>) -> Vec<SubMsg> {
    // BTreeMap used instead of HashMap for determinant ordering in tests. Both should work
    // on-chain as hashmap randomness is fixed in cosmwasm. We get O(logn) instead of O(1)
    // performance this way which is not a huge difference.
    let mut map: BTreeMap<String, Vec<Coin>> = BTreeMap::new();

    let mut merged_msgs: Vec<SubMsg> = vec![];
    for msg in msgs.into_iter() {
        match msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                let current_coins = map.get_mut(&to_address);
                match current_coins {
                    Some(current_coins) => merge_coins(current_coins, amount),
                    None => {
                        map.insert(to_address.to_owned(), amount);
                    }
                }
            }
            _ => merged_msgs.push(msg),
        }
    }

    for (to_address, amount) in map.into_iter() {
        merged_msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        })));
    }

    merged_msgs
}

/// Adds coins in `coins_to_add` to `coins` by merging those of the same denom and
/// otherwise appending.
///
/// ## Arguments
/// * `coins`        - Mutable reference to a vec of coins which will be modified in-place.
/// * `coins_to_add` - The `Vec<Coin>` to add, it is assumed that it contains no coins of the
///                    same denom
///
/// Returns nothing as it is done in place.
pub fn merge_coins(coins: &mut Vec<Coin>, coins_to_add: Vec<Coin>) {
    // Not the most efficient algorithm (O(n * m)) but we don't expect to deal with very large arrays of Coin,
    // typically at most 2 denoms. Even in the future there are not that many Terra native coins
    // where this will be a problem.
    for coin in coins.iter_mut() {
        let same_denom_coin = coins_to_add.iter().find(|&c| c.denom == coin.denom);
        if let Some(same_denom_coin) = same_denom_coin {
            coin.amount += same_denom_coin.amount;
        }
    }
    for coin_to_add in coins_to_add.iter() {
        if !coins.iter().any(|c| c.denom == coin_to_add.denom) {
            coins.push(coin_to_add.clone());
        }
    }
}

/// Deducts a given amount from a vector of `Coin` structs. Alters the given vector, does not return a new vector.
///
/// ## Arguments
/// * `coins` - The vector of `Coin` structs from which to deduct the given funds
/// * `funds` - The amount to deduct
pub fn deduct_funds(coins: &mut [Coin], funds: &Coin) -> Result<bool, ContractError> {
    let coin_amount = coins.iter_mut().find(|c| c.denom.eq(&funds.denom));

    match coin_amount {
        Some(c) => {
            ensure!(
                c.amount >= funds.amount,
                ContractError::InsufficientFunds {}
            );
            c.amount -= funds.amount;
            Ok(true)
        }
        None => Err(ContractError::InsufficientFunds {}),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{coin, to_binary, Uint128, WasmMsg};
    use cw20::Expiration;

    use super::*;

    #[cw_serde]
    struct TestStruct {
        name: String,
        expiration: Expiration,
    }

    #[test]
    fn test_parse_struct() {
        let valid_json = to_binary(&TestStruct {
            name: "John Doe".to_string(),
            expiration: Expiration::AtHeight(123),
        })
        .unwrap();

        let test_struct: TestStruct = parse_struct(&valid_json).unwrap();
        assert_eq!(test_struct.name, "John Doe");
        assert_eq!(test_struct.expiration, Expiration::AtHeight(123));

        let invalid_json = to_binary("notavalidteststruct").unwrap();

        assert!(parse_struct::<TestStruct>(&invalid_json).is_err())
    }

    #[test]
    fn test_merge_coins() {
        let mut coins = vec![coin(100, "uusd"), coin(100, "uluna")];
        let funds_to_add = vec![coin(25, "uluna"), coin(50, "uusd"), coin(100, "ucad")];

        merge_coins(&mut coins, funds_to_add);
        assert_eq!(
            vec![coin(150, "uusd"), coin(125, "uluna"), coin(100, "ucad")],
            coins
        );
    }

    #[test]
    fn test_merge_sub_messages() {
        let sub_msgs: Vec<SubMsg> = vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "A".to_string(),
                amount: vec![coin(100, "uusd"), coin(50, "uluna")],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "A".to_string(),
                amount: vec![coin(100, "uusd"), coin(50, "ukrw")],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "B".to_string(),
                amount: vec![coin(100, "uluna")],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "B".to_string(),
                amount: vec![coin(50, "uluna")],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "C".to_string(),
                funds: vec![],
                msg: encode_binary(&"").unwrap(),
            })),
        ];

        let merged_msgs = merge_sub_msgs(sub_msgs);
        assert_eq!(
            vec![
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "C".to_string(),
                    funds: vec![],
                    msg: encode_binary(&"").unwrap(),
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "A".to_string(),
                    amount: vec![coin(200, "uusd"), coin(50, "uluna"), coin(50, "ukrw")],
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "B".to_string(),
                    amount: vec![coin(150, "uluna")],
                })),
            ],
            merged_msgs
        );

        assert_eq!(3, merged_msgs.len());
    }

    #[test]
    fn test_deduct_funds() {
        let mut funds: Vec<Coin> = vec![coin(100, "uluna")];

        deduct_funds(&mut funds, &coin(10, "uluna")).unwrap();

        assert_eq!(Uint128::from(90_u64), funds[0].amount);
        assert_eq!(String::from("uluna"), funds[0].denom);

        let mut funds: Vec<Coin> = vec![Coin {
            denom: String::from("uluna"),
            amount: Uint128::from(5_u64),
        }];

        let e = deduct_funds(&mut funds, &coin(10, "uluna")).unwrap_err();

        assert_eq!(ContractError::InsufficientFunds {}, e);
    }
}
