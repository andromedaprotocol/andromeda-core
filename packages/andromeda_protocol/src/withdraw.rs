use cosmwasm_std::{coin, Deps, Env, MessageInfo, Order, Response, StdError, Storage, SubMsg};
use cw20::Cw20Coin;
use cw_storage_plus::Map;

use crate::{
    communication::Recipient, error::ContractError, operators::is_operator,
    ownership::is_contract_owner, require,
};
use terraswap::{
    asset::AssetInfo,
    querier::{query_balance, query_token_balance},
};

pub const WITHDRAWABLE_TOKENS: Map<&str, AssetInfo> = Map::new("withdrawable_tokens");

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
    recipient: Recipient,
    tokens_to_withdraw: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let sender = info.sender.as_str();
    require(
        is_contract_owner(deps.storage, sender)? || is_operator(deps.storage, sender)?,
        ContractError::Unauthorized {},
    )?;

    let keys = match tokens_to_withdraw {
        Some(tokens_to_withdraw) => tokens_to_withdraw,
        None => {
            let keys: Vec<Vec<u8>> = WITHDRAWABLE_TOKENS
                .keys(deps.storage, None, None, Order::Ascending)
                .collect();

            let res: Result<Vec<_>, _> =
                keys.iter().map(|v| String::from_utf8(v.to_vec())).collect();
            res.map_err(StdError::invalid_utf8)?
        }
    };

    let mut msgs: Vec<SubMsg> = vec![];

    for key in keys.iter() {
        let asset_info: AssetInfo = WITHDRAWABLE_TOKENS.load(deps.storage, key)?;
        let msg: Option<SubMsg> = match asset_info {
            AssetInfo::NativeToken { denom } => {
                let balance =
                    query_balance(&deps.querier, env.contract.address.clone(), denom.clone())?;
                if balance.is_zero() {
                    None
                } else {
                    let coin = coin(balance.u128(), denom);
                    Some(recipient.generate_msg_native(&deps, vec![coin])?)
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
                        amount: balance,
                    };
                    Some(recipient.generate_msg_cw20(&deps, cw20_coin)?)
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
            Recipient::Addr("address".to_string()),
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
            Recipient::Addr("address".to_string()),
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
            Recipient::Addr("address".to_string()),
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
            Recipient::Addr("address".to_string()),
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
            Recipient::Addr("address".to_string()),
            Some(vec!["uusd".to_string()]),
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
