use crate::state::ADOContract;
use andromeda_protocol::{
    communication::Recipient, error::ContractError, require, withdraw::Withdrawal,
};
use cosmwasm_std::{coin, Deps, Env, MessageInfo, Order, Response, StdError, Storage, SubMsg};
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
        deps: Deps,
        env: Env,
        info: MessageInfo,
        recipient: Option<Recipient>,
        tokens_to_withdraw: Option<Vec<Withdrawal>>,
    ) -> Result<Response, ContractError> {
        let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
        let sender = info.sender.as_str();
        require(
            self.is_contract_owner(deps.storage, sender)? || self.is_operator(deps.storage, sender),
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
                        Some(recipient.generate_msg_native(deps.api, vec![coin])?)
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
}
