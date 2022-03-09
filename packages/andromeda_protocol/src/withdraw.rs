use crate::{communication::Recipient, error::ContractError, require};
use cosmwasm_std::{
    coin, Deps, Env, MessageInfo, Order, Response, StdError, Storage, SubMsg, Uint128,
};
use cw20::Cw20Coin;
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::{
    asset::AssetInfo,
    querier::{query_balance, query_token_balance},
};

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

/*
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
                    contract_addr: MOCK_CW20_CONTRACT.into(),
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
}*/
