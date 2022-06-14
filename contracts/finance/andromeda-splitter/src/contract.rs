use crate::state::SPLITTER;
use ado_base::ADOContract;
use andromeda_finance::splitter::{
    validate_recipient_list, AddressPercent, ExecuteMsg, GetSplitterConfigResponse, InstantiateMsg,
    MigrateMsg, QueryMsg, Splitter,
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
    SubMsg, Timestamp, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::Expiration;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate()?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // Max 100 recipients
    require(
        msg.recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {},
    )?;
    let current_time = env.block.time.seconds();
    match msg.lock_time {
        Some(lock_time) => {
            // New lock time can't be too short (At least 1 day)
            require(lock_time >= 86400, ContractError::LockTimeTooShort {})?;

            // New lock time can't be too long (Max 1 year)
            require(lock_time <= 31_536_000, ContractError::LockTimeTooLong {})?;

            let splitter = Splitter {
                recipients: msg.recipients,
                lock: Expiration::AtTime(Timestamp::from_seconds(lock_time + current_time)),
            };
            SPLITTER.save(deps.storage, &splitter)?;
            ADOContract::default().instantiate(
                deps.storage,
                deps.api,
                info,
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    operators: None,
                    modules: msg.modules,
                    primitive_contract: None,
                },
            )
        }
        None => {
            let splitter = Splitter {
                recipients: msg.recipients,
                // If locking isn't desired upon instantiation, it's automatically set to 0
                lock: Expiration::AtTime(Timestamp::from_seconds(current_time)),
            };
            SPLITTER.save(deps.storage, &splitter)?;
            ADOContract::default().instantiate(
                deps.storage,
                deps.api,
                info,
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    operators: None,
                    modules: msg.modules,
                    primitive_contract: None,
                },
            )
        }
    }
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
            execute_update_recipients(deps, env, info, recipients)
        }
        ExecuteMsg::UpdateLock { lock_time } => execute_update_lock(deps, env, info, lock_time),
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
    // Looking at this nested for loop, we could find a way to reduce time/memory complexity to avoid DoS.
    // Would like to understand more about why we loop through funds and what it exactly stored in it.
    // From there we could look into HashMaps, or other methods to break the nested loops and avoid Denial of Service.
    // [ACK-04] Limit number of coins sent to 5.
    require(
        info.funds.len() < 5,
        ContractError::ExceedsMaxAllowedCoins {},
    )?;
    for recipient_addr in &splitter.recipients {
        let recipient_percent = recipient_addr.percent;
        let mut vec_coin: Vec<Coin> = Vec::new();
        for (i, coin) in sent_funds.iter().enumerate() {
            let mut recip_coin: Coin = coin.clone();
            recip_coin.amount = coin.amount * recipient_percent;
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
    // Who is the sender of this function?
    // Why does the remaining funds go the the sender of the executor of the splitter?
    // Is it considered tax(fee) or mistake?
    // Discussion around caller of splitter function in andromeda_splitter smart contract.
    // From tests, it looks like owner of smart contract (Andromeda) will recieve the rest of funds.
    // If so, should be documented
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
    env: Env,
    info: MessageInfo,
    recipients: Vec<AddressPercent>,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    validate_recipient_list(recipients.clone())?;

    let mut splitter = SPLITTER.load(deps.storage)?;
    // Can't call this function while the lock isn't expired

    require(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {},
    )?;
    // Max 100 recipients
    require(
        recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {},
    )?;

    splitter.recipients = recipients;
    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![attr("action", "update_recipients")]))
}

fn execute_update_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lock_time: u64,
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    // No need to send funds
    require(
        info.funds.is_empty(),
        ContractError::FunctionDeclinesFunds {},
    )?;

    let mut splitter = SPLITTER.load(deps.storage)?;

    // Can't call this function while the lock isn't expired

    require(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {},
    )?;
    // Get current time
    let current_time = env.block.time.seconds();

    // New lock time can't be too short (At least 1 day)
    require(lock_time >= 86400, ContractError::LockTimeTooShort {})?;

    // New lock time can't be unreasonably long (No more than 1 year)
    require(lock_time <= 31_536_000, ContractError::LockTimeTooLong {})?;

    // Set new lock time
    let new_lock = Expiration::AtTime(Timestamp::from_seconds(lock_time + current_time));

    splitter.lock = new_lock;

    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_lock"),
        attr("locked", new_lock.to_string()),
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
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
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
    use cosmwasm_std::{from_binary, Coin, Decimal};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            recipients: vec![AddressPercent {
                recipient: Recipient::from_string(String::from("Some Address")),
                percent: Decimal::one(),
            }],
            modules: None,
            lock_time: Some(0),
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_update_lock() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let current_time = env.block.time.seconds();
        let lock_time = 100_000;

        let owner = "creator";

        // Start off with an expiration that's behind current time (expired)
        let splitter = Splitter {
            recipients: vec![],
            lock: Expiration::AtTime(Timestamp::from_seconds(current_time - 1)),
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let msg = ExecuteMsg::UpdateLock { lock_time };
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

        let info = mock_info(owner, &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let new_lock = Expiration::AtTime(Timestamp::from_seconds(current_time + lock_time));
        assert_eq!(
            Response::default().add_attributes(vec![
                attr("action", "update_lock"),
                attr("locked", new_lock.to_string())
            ]),
            res
        );

        //check result
        let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
        assert!(!splitter.lock.is_expired(&env.block));
        assert_eq!(new_lock, splitter.lock);
    }

    #[test]
    fn test_execute_update_recipients() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let owner = "creator";

        let recipient = vec![
            AddressPercent {
                recipient: Recipient::from_string(String::from("addr1")),
                percent: Decimal::percent(40),
            },
            AddressPercent {
                recipient: Recipient::from_string(String::from("addr1")),
                percent: Decimal::percent(60),
            },
        ];
        let msg = ExecuteMsg::UpdateRecipients {
            recipients: recipient.clone(),
        };

        let splitter = Splitter {
            recipients: vec![],
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
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

        let sender_funds_amount = 10000u128;
        let owner = "creator";
        let info = mock_info(owner, &[Coin::new(sender_funds_amount, "uluna")]);

        let recip_address1 = "address1".to_string();
        let recip_percent1 = 10; // 10%

        let recip_address2 = "address2".to_string();
        let recip_percent2 = 20; // 20%

        let recipient = vec![
            AddressPercent {
                recipient: Recipient::from_string(recip_address1.clone()),
                percent: Decimal::percent(recip_percent1),
            },
            AddressPercent {
                recipient: Recipient::from_string(recip_address2.clone()),
                percent: Decimal::percent(recip_percent2),
            },
        ];
        let msg = ExecuteMsg::Send {};

        let splitter = Splitter {
            recipients: recipient,
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
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

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected_res = Response::new()
            .add_submessages(vec![
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: recip_address1,
                    amount: vec![Coin::new(1000, "uluna")], // 10000 * 0.1
                })),
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: recip_address2,
                    amount: vec![Coin::new(2000, "uluna")], // 10000 * 0.2
                })),
                SubMsg::new(
                    // refunds remainder to sender
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: owner.to_string(),
                        amount: vec![Coin::new(7000, "uluna")], // 10000 * 0.7   remainder
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
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let query_msg = QueryMsg::GetSplitterConfig {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetSplitterConfigResponse = from_binary(&res).unwrap();

        assert_eq!(val.config, splitter);
    }

    #[test]
    fn test_execute_send_error() {
        //Executes send with more than 5 tokens [ACK-04]
        let mut deps = mock_dependencies();
        let env = mock_env();

        let sender_funds_amount = 10000u128;
        let owner = "creator";
        let info = mock_info(
            owner,
            &vec![
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
                Coin::new(sender_funds_amount, "uluna"),
            ],
        );

        let recip_address1 = "address1".to_string();
        let recip_percent1 = 10; // 10%

        let recip_address2 = "address2".to_string();
        let recip_percent2 = 20; // 20%

        let recipient = vec![
            AddressPercent {
                recipient: Recipient::from_string(recip_address1),
                percent: Decimal::percent(recip_percent1),
            },
            AddressPercent {
                recipient: Recipient::from_string(recip_address2),
                percent: Decimal::percent(recip_percent2),
            },
        ];
        let msg = ExecuteMsg::Send {};

        let splitter = Splitter {
            recipients: recipient,
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
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

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

        let expected_res = ContractError::ExceedsMaxAllowedCoins {};

        assert_eq!(res, expected_res);
    }
}
