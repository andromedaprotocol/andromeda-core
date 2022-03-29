use crate::ADOContract;
use common::{ado_base::recipient::Recipient, error::ContractError, require, withdraw::Withdrawal};
use cosmwasm_std::{coin, DepsMut, Env, MessageInfo, Order, Response, StdError, Storage, SubMsg};
use cw20::Cw20Coin;

use terraswap::{
    asset::AssetInfo,
    querier::{query_balance, query_token_balance},
};

impl<'a> ADOContract<'a> {
    pub fn add_withdrawable_token(
        &self,
        storage: &mut dyn Storage,
        name: &str,
        asset_info: &AssetInfo,
    ) -> Result<(), ContractError> {
        if !self.withdrawable_tokens.has(storage, name) {
            self.withdrawable_tokens.save(storage, name, asset_info)?;
        }
        Ok(())
    }

    pub fn remove_withdrawable_token(
        &self,
        storage: &mut dyn Storage,
        name: &str,
    ) -> Result<(), ContractError> {
        self.withdrawable_tokens.remove(storage, name);
        Ok(())
    }

    /// Withdraw all tokens in self.withdrawable_tokens with non-zero balance to the given recipient.
    pub fn execute_withdraw(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: Option<Recipient>,
        tokens_to_withdraw: Option<Vec<Withdrawal>>,
    ) -> Result<Response, ContractError> {
        let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
        let sender = info.sender.as_str();
        require(
            self.is_owner_or_operator(deps.storage, sender)?,
            ContractError::Unauthorized {},
        )?;

        let withdrawals = match tokens_to_withdraw {
            Some(tokens_to_withdraw) => tokens_to_withdraw,
            None => {
                let keys: Vec<Vec<u8>> = self
                    .withdrawable_tokens
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
            let asset_info: AssetInfo = self
                .withdrawable_tokens
                .load(deps.storage, &withdrawal.token)?;
            let msg: Option<SubMsg> = match asset_info {
                AssetInfo::NativeToken { denom } => {
                    let balance =
                        query_balance(&deps.querier, env.contract.address.clone(), denom.clone())?;
                    if balance.is_zero() {
                        None
                    } else {
                        let coin = coin(withdrawal.get_amount(balance)?.u128(), denom);
                        Some(recipient.generate_msg_native(
                            deps.api,
                            &deps.querier,
                            self.mission_contract.may_load(deps.storage)?,
                            vec![coin],
                        )?)
                    }
                }
                AssetInfo::Token { contract_addr } => {
                    let balance = query_token_balance(
                        &deps.querier,
                        deps.api.addr_validate(&contract_addr)?,
                        env.contract.address.clone(),
                    )?;
                    if balance.is_zero() {
                        None
                    } else {
                        let cw20_coin = Cw20Coin {
                            address: contract_addr,
                            amount: withdrawal.get_amount(balance)?,
                        };
                        Some(recipient.generate_msg_cw20(
                            deps.api,
                            &deps.querier,
                            self.mission_contract.may_load(deps.storage)?,
                            cw20_coin,
                        )?)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{mock_dependencies_custom, MOCK_CW20_CONTRACT};
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary, Addr, BankMsg, CosmosMsg, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;

    #[test]
    fn test_execute_withdraw_not_authorized() {
        let mut deps = mock_dependencies(&[]);
        let owner = "owner";
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        let info = mock_info("not_owner", &[]);
        let res = ADOContract::default().execute_withdraw(
            deps.as_mut(),
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
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        let info = mock_info(owner, &[]);
        let res = ADOContract::default().execute_withdraw(
            deps.as_mut(),
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
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        let info = mock_info(owner, &[]);
        ADOContract::default()
            .withdrawable_tokens
            .save(
                deps.as_mut().storage,
                "uusd",
                &AssetInfo::NativeToken {
                    denom: "uusd".into(),
                },
            )
            .unwrap();
        let res = ADOContract::default()
            .execute_withdraw(
                deps.as_mut(),
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
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked("owner"))
            .unwrap();
        ADOContract::default()
            .operators
            .save(deps.as_mut().storage, operator, &true)
            .unwrap();
        let info = mock_info(operator, &[]);
        ADOContract::default()
            .withdrawable_tokens
            .save(
                deps.as_mut().storage,
                MOCK_CW20_CONTRACT,
                &AssetInfo::Token {
                    contract_addr: MOCK_CW20_CONTRACT.into(),
                },
            )
            .unwrap();
        let res = ADOContract::default()
            .execute_withdraw(
                deps.as_mut(),
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
        ADOContract::default()
            .owner
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();
        let info = mock_info(owner, &[]);
        ADOContract::default()
            .withdrawable_tokens
            .save(
                deps.as_mut().storage,
                "uusd",
                &AssetInfo::NativeToken {
                    denom: "uusd".into(),
                },
            )
            .unwrap();
        ADOContract::default()
            .withdrawable_tokens
            .save(
                deps.as_mut().storage,
                "uluna",
                &AssetInfo::NativeToken {
                    denom: "uluna".into(),
                },
            )
            .unwrap();
        let res = ADOContract::default()
            .execute_withdraw(
                deps.as_mut(),
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
