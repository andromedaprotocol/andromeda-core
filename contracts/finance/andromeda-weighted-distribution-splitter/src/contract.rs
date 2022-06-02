use crate::state::SPLITTER;

use ado_base::ADOContract;
use andromeda_finance::weighted_splitter::{
    validate_recipient_list, AddressWeight, ExecuteMsg, GetSplitterConfigResponse,
    GetUserWeightResponse, InstantiateMsg, MigrateMsg, QueryMsg, Splitter,
};
use common::{
    ado_base::{
        hooks::AndromedaHook, recipient::Recipient, AndromedaMsg,
        InstantiateMsg as BaseInstantiateMsg,
    },
    app::AndrAddress,
    encode_binary,
    error::ContractError,
    require,
};

use cosmwasm_std::{
    attr, entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, SubMsg, Uint128,
};

use cw2::{get_contract_version, set_contract_version};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-weighted-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate()?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let splitter = Splitter {
        recipients: msg.recipients,
        locked: false,
    };

    SPLITTER.save(deps.storage, &splitter)?;
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "weighted-splitter".to_string(),
            operators: None,
            modules: msg.modules,
            primitive_contract: None,
        },
    )
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    // Do this before the hooks get fired off to ensure that there is no conflict with the app
    // contract not being whitelisted.
    if let ExecuteMsg::AndrReceive(AndromedaMsg::UpdateAppContract { address }) = msg {
        let splitter = SPLITTER.load(deps.storage)?;
        let mut andr_addresses: Vec<AndrAddress> = vec![];
        for recipient in splitter.recipients {
            if let Recipient::ADO(ado_recipient) = recipient.recipient {
                andr_addresses.push(ado_recipient.address);
            }
        }
        return contract.execute_update_app_contract(deps, info, address, Some(andr_addresses));
    };

    contract.module_hook::<Response>(
        deps.storage,
        deps.api,
        deps.querier,
        AndromedaHook::OnExecute {
            sender: info.sender.to_string(),
            payload: encode_binary(&msg)?,
        },
    )?;

    match msg {
        ExecuteMsg::UpdateRecipients { recipients } => {
            execute_update_recipients(deps, info, recipients)
        }
        ExecuteMsg::RemoveRecipient { recipient } => {
            execute_remove_recipient(deps, info, recipient)
        }
        ExecuteMsg::UpdateLock { lock } => execute_update_lock(deps, info, lock),

        ExecuteMsg::Send {} => execute_send(deps, info),
        ExecuteMsg::AndrReceive(msg) => execute_andromeda(deps, env, info, msg),
    }
}

pub fn execute_andromeda(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(..) => execute_send(deps, info),
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

fn execute_send(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let sent_funds: Vec<Coin> = info.funds.clone();
    require(
        !sent_funds.is_empty(),
        ContractError::InvalidFunds {
            msg: "Require at least one coin to be sent".to_string(),
        },
    )?;

    let splitter = SPLITTER.load(deps.storage)?;
    let mut msgs: Vec<SubMsg> = Vec::new();

    let mut remainder_funds = info.funds.clone();

    require(
        info.funds.len() < 5,
        ContractError::ExceedsMaxAllowedCoins {},
    )?;
    let mut total_weight = Uint128::new(0);

    // Calculate the total weight
    for recipient_addr in &splitter.recipients {
        let recipient_weight = recipient_addr.weight;
        total_weight += recipient_weight;
    }

    for recipient_addr in &splitter.recipients {
        let recipient_weight = recipient_addr.weight;
        let mut vec_coin: Vec<Coin> = Vec::new();
        for (i, coin) in sent_funds.iter().enumerate() {
            let mut recip_coin: Coin = coin.clone();
            recip_coin.amount = coin.amount.multiply_ratio(recipient_weight, total_weight);
            remainder_funds[i].amount -= recip_coin.amount;
            vec_coin.push(recip_coin);
        }
        // ADO receivers must use AndromedaMsg::Receive to execute their functionality
        // Others may just receive the funds
        let msg = recipient_addr.recipient.generate_msg_native(
            deps.api,
            &deps.querier,
            ADOContract::default().get_app_contract(deps.storage)?,
            vec_coin,
        )?;
        msgs.push(msg);
    }
    remainder_funds = remainder_funds
        .into_iter()
        .filter(|x| x.amount > Uint128::zero())
        .collect();

    if !remainder_funds.is_empty() {
        msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: remainder_funds,
        })));
    }

    Ok(Response::new()
        .add_submessages(msgs)
        .add_attributes(vec![attr("action", "send"), attr("sender", info.sender)]))
}

fn execute_update_recipients(
    deps: DepsMut,
    info: MessageInfo,
    recipients: Vec<AddressWeight>,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    validate_recipient_list(recipients.clone())?;

    let mut splitter = SPLITTER.load(deps.storage)?;
    if splitter.locked {
        StdError::generic_err("The splitter is currently locked");
    }

    splitter.recipients = recipients;
    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![attr("action", "update_recipients")]))
}

fn execute_remove_recipient(
    deps: DepsMut,
    info: MessageInfo,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let mut splitter = SPLITTER.load(deps.storage)?;
    if splitter.locked {
        StdError::generic_err("The splitter is currently locked");
    }

    let user_index = splitter
        .recipients
        .clone()
        .into_iter()
        .position(|x| x.recipient == recipient);

    if let Some(i) = user_index {
        splitter.recipients.swap_remove(i);
    } else {
        StdError::generic_err("User not found");
    };

    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![attr("action", "removed_recipient")]))
}

fn execute_update_lock(
    deps: DepsMut,
    info: MessageInfo,
    lock: bool,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let mut splitter = SPLITTER.load(deps.storage)?;
    splitter.locked = lock;
    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_lock"),
        attr("locked", lock.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetSplitterConfig {} => encode_binary(&query_splitter(deps)?),
        QueryMsg::GetUserWeight { user } => encode_binary(&query_user_weight(deps, user)?),
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
    }
}

fn query_user_weight(deps: Deps, user: Recipient) -> Result<GetUserWeightResponse, ContractError> {
    let splitter = SPLITTER.load(deps.storage)?;
    let recipients = splitter.recipients;

    let addrs: Vec<AddressWeight> = recipients
        .into_iter()
        .filter(|x| x.recipient == user)
        .collect();

    // Check if the address exists in the list. If it exists, extract the weight.
    if addrs.is_empty() {
        Ok(GetUserWeightResponse {
            weight: Uint128::new(0),
        })
    } else {
        let weight = addrs[0].weight;
        // There should be only one element at index 0
        Ok(GetUserWeightResponse { weight })
    }
}

fn query_splitter(deps: Deps) -> Result<GetSplitterConfigResponse, ContractError> {
    let splitter = SPLITTER.load(deps.storage)?;

    Ok(GetSplitterConfigResponse { config: splitter })
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ado_base::recipient::Recipient;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Coin};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            recipients: vec![AddressWeight {
                recipient: Recipient::from_string(String::from("Some Address")),
                weight: Uint128::new(1),
            }],
            modules: None,
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_update_lock() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let owner = "creator";

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let lock = true;
        let msg = ExecuteMsg::UpdateLock { lock };
        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    operators: None,
                    modules: None,
                    primitive_contract: None,
                },
            )
            .unwrap();

        let info = mock_info("incorrect_owner", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

        let info = mock_info(owner, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            Response::default().add_attributes(vec![
                attr("action", "update_lock"),
                attr("locked", lock.to_string())
            ]),
            res
        );

        //check result
        let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
        assert_eq!(splitter.locked, lock);
    }

    #[test]
    fn test_execute_remove_recipient() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let owner = "creator";

        let recipient = vec![
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr1")),
                weight: Uint128::new(40),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr2")),
                weight: Uint128::new(60),
            },
        ];
        let msg = ExecuteMsg::UpdateRecipients {
            recipients: recipient.clone(),
        };

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    operators: None,
                    modules: None,
                    primitive_contract: None,
                },
            )
            .unwrap();

        let info = mock_info("incorrect_owner", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

        let info = mock_info(owner, &[]);

        let msg = ExecuteMsg::UpdateRecipients {
            recipients: recipient.clone(),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::RemoveRecipient {
            recipient: Recipient::from_string(String::from("addr2")),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            Response::default().add_attributes(vec![attr("action", "removed_recipient")]),
            res
        );

        // check result
        let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            splitter.recipients[0],
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr1")),
                weight: Uint128::new(40),
            }
        );
    }

    #[test]
    fn test_execute_update_recipients() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let owner = "creator";

        let recipient = vec![
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr1")),
                weight: Uint128::new(40),
            },
            AddressWeight {
                recipient: Recipient::from_string(String::from("addr2")),
                weight: Uint128::new(60),
            },
        ];
        let msg = ExecuteMsg::UpdateRecipients {
            recipients: recipient.clone(),
        };

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    operators: None,
                    modules: None,
                    primitive_contract: None,
                },
            )
            .unwrap();

        let info = mock_info("incorrect_owner", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

        let info = mock_info(owner, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            Response::default().add_attributes(vec![attr("action", "update_recipients")]),
            res
        );

        //check result
        let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
        assert_eq!(splitter.recipients, recipient);
    }

    #[test]
    fn test_execute_send() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let sender_funds_amount = 10000_u128;
        let owner = "creator";

        let recip_address1 = "address1".to_string();
        let recip_weight1 = Uint128::new(10); // Weight of 10

        let recip_address2 = "address2".to_string();
        let recip_percent2 = Uint128::new(20); // Weight of 20

        let recipient = vec![
            AddressWeight {
                recipient: Recipient::Addr(recip_address1.clone()),
                weight: recip_weight1,
            },
            AddressWeight {
                recipient: Recipient::Addr(recip_address2.clone()),
                weight: recip_percent2,
            },
        ];
        let msg = ExecuteMsg::Send {};

        //incorrect owner
        let info = mock_info("incorrect_owner", &[]);

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_ret) => assert!(false),
            _ => {}
        }

        let splitter = Splitter {
            recipients: recipient,
            locked: false,
        };

        let info = mock_info(owner.clone(), &vec![Coin::new(10000_u128, "uluna")]);
        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                deps_mut.api,
                info.clone(),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    operators: None,
                    modules: None,
                    primitive_contract: None,
                },
            )
            .unwrap();

        SPLITTER.save(deps_mut.storage, &splitter).unwrap();

        let res = execute(deps_mut, env, info, msg).unwrap();

        let expected_res = Response::new()
            .add_submessages(vec![
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: recip_address1,
                    amount: vec![Coin::new(3333, "uluna")], // 10000 * (10/30)
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: recip_address2,
                    amount: vec![Coin::new(6666, "uluna")], // 10000 * (20/30)
                })),
                SubMsg::new(
                    // refunds remainder to sender
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: owner.to_string(),
                        amount: vec![Coin::new(1, "uluna")], // 10000 - (3333+6666)   remainder
                    }),
                ),
            ])
            .add_attributes(vec![attr("action", "send"), attr("sender", "creator")]);

        assert_eq!(res, expected_res);
    }

    #[test]
    fn test_query_splitter() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let splitter = Splitter {
            recipients: vec![],
            locked: false,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let query_msg = QueryMsg::GetSplitterConfig {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetSplitterConfigResponse = from_binary(&res).unwrap();

        assert_eq!(val.config, splitter);
    }

    #[test]
    fn test_query_user_weight() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let user1 = AddressWeight {
            recipient: Recipient::Addr("first".to_string()),
            weight: Uint128::new(5),
        };
        let user2 = AddressWeight {
            recipient: Recipient::Addr("second".to_string()),
            weight: Uint128::new(10),
        };
        let splitter = Splitter {
            recipients: vec![user1, user2],
            locked: false,
        };

        SPLITTER
            .save(deps.as_mut().storage, &splitter.clone())
            .unwrap();

        let query_msg = QueryMsg::GetUserWeight {
            user: Recipient::Addr("second".to_string()),
        };
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetUserWeightResponse = from_binary(&res).unwrap();

        assert_eq!(val.weight, Uint128::new(10));
    }

    #[test]
    fn test_execute_send_error() {
        //Executes send with more than 5 tokens [ACK-04]
        let mut deps = mock_dependencies();
        let env = mock_env();

        let sender_funds_amount = 10000u128;
        let owner = "creator";

        let recip_address1 = "address1".to_string();
        let recip_weight1 = Uint128::new(10); // Weight of 10

        let recip_address2 = "address2".to_string();
        let recip_weight2 = Uint128::new(20); // Weight of 20

        let recipient = vec![
            AddressWeight {
                recipient: Recipient::Addr(recip_address1),
                weight: recip_weight1,
            },
            AddressWeight {
                recipient: Recipient::Addr(recip_address2),
                weight: recip_weight2,
            },
        ];
        let msg = ExecuteMsg::Send {};

        //incorrect owner
        let info = mock_info("incorrect_owner", &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_ret) => assert!(false),
            _ => {}
        }

        let info = mock_info(
            owner.clone(),
            &vec![
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
            ],
        );
        let splitter = Splitter {
            recipients: recipient,
            locked: false,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

        let expected_res = ContractError::ExceedsMaxAllowedCoins {};

        assert_eq!(res, expected_res);
    }
}
