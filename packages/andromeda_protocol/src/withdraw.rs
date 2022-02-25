use crate::{
    communication::Recipient,
    error::ContractError,
    operators::is_operator,
    ownership::is_contract_owner,
    require,
    swapper::{query_balance, query_token_balance, AssetInfo},
};
use cosmwasm_std::{
    coin, Deps, Env, MessageInfo, Order, Response, StdError, Storage, SubMsg, Uint128,
};
use cw20::Cw20Coin;
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const WITHDRAWABLE_TOKENS: Map<&str, AssetInfo> = Map::new("withdrawable_tokens");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Withdrawal {
    pub token: String,
    pub withdrawal_type: Option<WithdrawalType>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WithdrawalType {
    Amount(Uint128),
    Percentage(Uint128),
}

impl Withdrawal {
    /// Calculates the amount to withdraw given the withdrawal type and passed in `balance`.
    pub fn get_amount(&self, balance: Uint128) -> Result<Uint128, ContractError> {
        match self.withdrawal_type.clone() {
            None => Ok(balance),
            Some(withdrawal_type) => match withdrawal_type {
                WithdrawalType::Percentage(percent) => {
                    require(percent <= 100u128.into(), ContractError::InvalidRate {})?;
                    Ok(balance.multiply_ratio(percent, 100u128))
                }
                WithdrawalType::Amount(amount) => {
                    require(
                        amount <= balance,
                        ContractError::InvalidFunds {
                            msg: "Requested withdrawal amount exceeds token balance".to_string(),
                        },
                    )?;
                    Ok(amount)
                }
            },
        }
    }
}

pub fn add_withdrawable_token(
    storage: &mut dyn Storage,
    name: &str,
    asset_info: &AssetInfo,
) -> Result<(), ContractError> {
    if !WITHDRAWABLE_TOKENS.has(storage, name) {
        WITHDRAWABLE_TOKENS.save(storage, name, asset_info)?;
    }
    Ok(())
}

pub fn remove_withdrawable_token(
    storage: &mut dyn Storage,
    name: &str,
) -> Result<(), ContractError> {
    WITHDRAWABLE_TOKENS.remove(storage, name);
    Ok(())
}

/// Withdraw all tokens in WITHDRAWABLE_TOKENS with non-zero balance to the given recipient.
pub fn execute_withdraw(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    recipient: Option<Recipient>,
    tokens_to_withdraw: Option<Vec<Withdrawal>>,
) -> Result<Response, ContractError> {
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
    let sender = info.sender.as_str();
    require(
        is_contract_owner(deps.storage, sender)? || is_operator(deps.storage, sender)?,
        ContractError::Unauthorized {},
    )?;

    let withdrawals = match tokens_to_withdraw {
        Some(tokens_to_withdraw) => tokens_to_withdraw,
        None => {
            let keys: Vec<Vec<u8>> = WITHDRAWABLE_TOKENS
                .keys(deps.storage, None, None, Order::Ascending)
                .collect();

            let res: Result<Vec<_>, _> =
                keys.iter().map(|v| String::from_utf8(v.to_vec())).collect();

            let res = res.map_err(StdError::invalid_utf8)?;

            res.iter()
                .map(|k| Withdrawal {
                    token: k.to_string(),
                    withdrawal_type: None,
                })
                .collect()
        }
    };

    let mut msgs: Vec<SubMsg> = vec![];

    for withdrawal in withdrawals.iter() {
        let asset_info: AssetInfo = WITHDRAWABLE_TOKENS.load(deps.storage, &withdrawal.token)?;
        let msg: Option<SubMsg> = match asset_info {
            AssetInfo::NativeToken { denom } => {
                let balance =
                    query_balance(&deps.querier, env.contract.address.clone(), denom.clone())?;
                if balance.is_zero() {
                    None
                } else {
                    let coin = coin(withdrawal.get_amount(balance)?.u128(), denom);
                    Some(recipient.generate_msg_native(deps.api, vec![coin])?)
                }
            }
            AssetInfo::Token { contract_addr } => {
                let balance = query_token_balance(
                    &deps.querier,
                    contract_addr.clone(),
                    env.contract.address.clone(),
                )?;
                if balance.is_zero() {
                    None
                } else {
                    let cw20_coin = Cw20Coin {
                        address: contract_addr.to_string(),
                        amount: withdrawal.get_amount(balance)?,
                    };
                    Some(recipient.generate_msg_cw20(deps.api, cw20_coin)?)
                }
            }
        };
        if let Some(msg) = msg {
            msgs.push(msg);
        }
    }
    require(
        !msgs.is_empty(),
        ContractError::InvalidFunds {
            msg: "No funds to withdraw".to_string(),
        },
    )?;
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "withdraw")
        .add_attribute("recipient", format!("{:?}", recipient)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        operators::OPERATORS,
        ownership::CONTRACT_OWNER,
        testing::mock_querier::{mock_dependencies_custom, MOCK_CW20_CONTRACT},
    };
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{to_binary, Addr, BankMsg, CosmosMsg, SubMsg, WasmMsg};
    use cw20::Cw20ExecuteMsg;

    #[test]
    fn test_get_amount_no_withdrawal_type() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: None,
        };
        let balance = Uint128::from(100u128);
        assert_eq!(balance, withdrawal.get_amount(balance).unwrap());
    }

    #[test]
    fn test_get_amount_percentage() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Percentage(10u128.into())),
        };
        let balance = Uint128::from(100u128);
        assert_eq!(10u128, withdrawal.get_amount(balance).unwrap().u128());
    }

    #[test]
    fn test_get_amount_invalid_percentage() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Percentage(101u128.into())),
        };
        let balance = Uint128::from(100u128);
        assert_eq!(
            ContractError::InvalidRate {},
            withdrawal.get_amount(balance).unwrap_err()
        );
    }

    #[test]
    fn test_get_amount_amount() {
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Amount(5u128.into())),
        };
        let balance = Uint128::from(10u128);
        assert_eq!(5u128, withdrawal.get_amount(balance).unwrap().u128());
    }

    #[test]
    fn test_get_invalid_amount() {
        let balance = Uint128::from(10u128);
        let withdrawal = Withdrawal {
            token: "token".to_string(),
            withdrawal_type: Some(WithdrawalType::Amount(balance + Uint128::from(1u128))),
        };
        assert_eq!(
            ContractError::InvalidFunds {
                msg: "Requested withdrawal amount exceeds token balance".to_string(),
            },
            withdrawal.get_amount(balance).unwrap_err()
        );
    }

    #[test]
    fn test_execute_withdraw_not_authorized() {
        let mut deps = mock_dependencies(&[]);
        let owner = "owner";
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        let info = mock_info("not_owner", &[]);
        let res = execute_withdraw(
            deps.as_ref(),
            mock_env(),
            info,
            Some(Recipient::Addr("address".to_string())),
            None,
        );
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_withdraw_no_funds() {
        let mut deps = mock_dependencies(&[]);
        let owner = "owner";
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        let info = mock_info(owner, &[]);
        let res = execute_withdraw(
            deps.as_ref(),
            mock_env(),
            info,
            Some(Recipient::Addr("address".to_string())),
            None,
        );
        assert_eq!(
            ContractError::InvalidFunds {
                msg: "No funds to withdraw".to_string(),
            },
            res.unwrap_err()
        );
    }

    #[test]
    fn test_execute_withdraw_native() {
        let mut deps = mock_dependencies(&[coin(100, "uusd")]);
        let owner = "owner";
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        let info = mock_info(owner, &[]);
        WITHDRAWABLE_TOKENS
            .save(
                deps.as_mut().storage,
                "uusd",
                &AssetInfo::NativeToken {
                    denom: "uusd".into(),
                },
            )
            .unwrap();
        let res = execute_withdraw(
            deps.as_ref(),
            mock_env(),
            info,
            Some(Recipient::Addr("address".to_string())),
            None,
        )
        .unwrap();
        let msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "address".to_string(),
            amount: vec![coin(100, "uusd")],
        }));
        assert_eq!(
            Response::new()
                .add_submessage(msg)
                .add_attribute("action", "withdraw")
                .add_attribute("recipient", "Addr(\"address\")"),
            res
        );
    }

    #[test]
    fn test_execute_withdraw_cw20() {
        let mut deps = mock_dependencies_custom(&[]);
        let operator = "operator";
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();
        OPERATORS
            .save(deps.as_mut().storage, operator, &true)
            .unwrap();
        let info = mock_info(operator, &[]);
        WITHDRAWABLE_TOKENS
            .save(
                deps.as_mut().storage,
                MOCK_CW20_CONTRACT,
                &AssetInfo::Token {
                    contract_addr: Addr::unchecked(MOCK_CW20_CONTRACT),
                },
            )
            .unwrap();
        let res = execute_withdraw(
            deps.as_ref(),
            mock_env(),
            info,
            Some(Recipient::Addr("address".to_string())),
            None,
        )
        .unwrap();
        let msg = SubMsg::new(WasmMsg::Execute {
            contract_addr: MOCK_CW20_CONTRACT.into(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "address".to_string(),
                amount: 10u128.into(),
            })
            .unwrap(),
            funds: vec![],
        });
        assert_eq!(
            Response::new()
                .add_submessage(msg)
                .add_attribute("action", "withdraw")
                .add_attribute("recipient", "Addr(\"address\")"),
            res
        );
    }

    #[test]
    fn test_execute_withdraw_selective() {
        let mut deps = mock_dependencies(&[coin(100, "uusd"), coin(100, "uluna")]);
        let owner = "owner";
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        let info = mock_info(owner, &[]);
        WITHDRAWABLE_TOKENS
            .save(
                deps.as_mut().storage,
                "uusd",
                &AssetInfo::NativeToken {
                    denom: "uusd".into(),
                },
            )
            .unwrap();
        WITHDRAWABLE_TOKENS
            .save(
                deps.as_mut().storage,
                "uluna",
                &AssetInfo::NativeToken {
                    denom: "uluna".into(),
                },
            )
            .unwrap();
        let res = execute_withdraw(
            deps.as_ref(),
            mock_env(),
            info,
            Some(Recipient::Addr("address".to_string())),
            Some(vec![Withdrawal {
                token: "uusd".to_string(),
                withdrawal_type: None,
            }]),
        )
        .unwrap();
        let msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "address".to_string(),
            amount: vec![coin(100, "uusd")],
        }));
        assert_eq!(
            Response::new()
                .add_submessage(msg)
                .add_attribute("action", "withdraw")
                .add_attribute("recipient", "Addr(\"address\")"),
            res
        );
    }
}
