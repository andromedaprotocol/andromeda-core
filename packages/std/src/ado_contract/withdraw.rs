use crate::ado_contract::ADOContract;
use crate::{ado_base::withdraw::Withdrawal, amp::recipient::Recipient, error::ContractError};
use cosmwasm_std::{
    coin, ensure, DepsMut, Env, MessageInfo, Order, Response, StdError, Storage, SubMsg,
};
use cw20::Cw20Coin;

use cw_asset::AssetInfo;

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
        let recipient =
            recipient.unwrap_or_else(|| Recipient::from_string(info.sender.to_string()));
        let sender = info.sender.as_str();
        ensure!(
            self.is_owner_or_operator(deps.storage, sender)?,
            ContractError::Unauthorized {}
        );

        let withdrawals = match tokens_to_withdraw {
            Some(tokens_to_withdraw) => tokens_to_withdraw,
            None => {
                let withdrawals: Vec<Withdrawal> = self
                    .withdrawable_tokens
                    .keys(deps.storage, None, None, Order::Ascending)
                    .map(|token| {
                        Ok(Withdrawal {
                            token: token?,
                            withdrawal_type: None,
                        })
                    })
                    .collect::<Result<Vec<Withdrawal>, StdError>>()?;

                withdrawals
            }
        };

        let mut msgs: Vec<SubMsg> = vec![];

        for withdrawal in withdrawals.iter() {
            let asset_info: AssetInfo = self
                .withdrawable_tokens
                .load(deps.storage, &withdrawal.token)?;
            let msg: Option<SubMsg> = match &asset_info {
                AssetInfo::Native(denom) => {
                    let balance =
                        asset_info.query_balance(&deps.querier, env.contract.address.clone())?;
                    if balance.is_zero() {
                        None
                    } else {
                        let coin = coin(withdrawal.get_amount(balance)?.u128(), denom);
                        Some(recipient.generate_direct_msg(&deps.as_ref(), vec![coin])?)
                    }
                }
                AssetInfo::Cw20(contract_addr) => {
                    let contract_addr_str = contract_addr.to_string();
                    let balance =
                        asset_info.query_balance(&deps.querier, env.contract.address.clone())?;
                    if balance.is_zero() {
                        None
                    } else {
                        let cw20_coin = Cw20Coin {
                            address: contract_addr_str,
                            amount: withdrawal.get_amount(balance)?,
                        };
                        Some(recipient.generate_msg_cw20(&deps.as_ref(), cw20_coin)?)
                    }
                }
                &_ => Err(ContractError::InvalidFunds {
                    msg: "Invalid asset info".to_string(),
                })?,
            };
            if let Some(msg) = msg {
                msgs.push(msg);
            }
        }
        ensure!(
            !msgs.is_empty(),
            ContractError::InvalidFunds {
                msg: "No funds to withdraw".to_string(),
            }
        );
        Ok(Response::new()
            .add_submessages(msgs)
            .add_attribute("action", "withdraw")
            .add_attribute("recipient", format!("{:?}", recipient)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock_querier::{mock_dependencies_custom, MOCK_CW20_CONTRACT};
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary, Addr, BankMsg, CosmosMsg, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;

    #[test]
    fn test_execute_withdraw_not_authorized() {
        let mut deps = mock_dependencies();
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
            Some(Recipient::from_string("address".to_string())),
            None,
        );
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_execute_withdraw_no_funds() {
        let mut deps = mock_dependencies();
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
            Some(Recipient::from_string("address".to_string())),
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
        let mut deps = mock_dependencies();
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
                &AssetInfo::Native("uusd".into()),
            )
            .unwrap();
        let res = ADOContract::default()
            .execute_withdraw(
                deps.as_mut(),
                mock_env(),
                info,
                Some(Recipient::from_string("address".to_string())),
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
                &AssetInfo::Cw20(Addr::unchecked(MOCK_CW20_CONTRACT)),
            )
            .unwrap();
        let res = ADOContract::default()
            .execute_withdraw(
                deps.as_mut(),
                mock_env(),
                info,
                Some(Recipient::from_string("address".to_string())),
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
        let mut deps = mock_dependencies();
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
                &AssetInfo::Native("uusd".into()),
            )
            .unwrap();
        ADOContract::default()
            .withdrawable_tokens
            .save(
                deps.as_mut().storage,
                "uluna",
                &AssetInfo::Native("uluna".into()),
            )
            .unwrap();
        let res = ADOContract::default()
            .execute_withdraw(
                deps.as_mut(),
                mock_env(),
                info,
                Some(Recipient::from_string("address".to_string())),
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
