use crate::state::SPLITTER;
use ado_base::ADOContract;
use andromeda_finance::splitter::{
    validate_recipient_list, AddressPercent, ExecuteMsg, GetSplitterConfigResponse, InstantiateMsg,
    MigrateMsg, QueryMsg, Splitter,
};
use andromeda_os::recipient::generate_msg_native_kernel;

use andromeda_os::messages::{AMPMsg, AMPPkt, ReplyGasExit};
use common::{
    ado_base::{hooks::AndromedaHook, AndromedaMsg, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::{from_semver, ContractError},
};
use cosmwasm_std::{
    attr, ensure, entry_point, from_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, ReplyOn, Response, SubMsg, Timestamp, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::{nonpayable, Expiration};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
// 1 day in seconds
const ONE_DAY: u64 = 86_400;
// 1 year in seconds
const ONE_YEAR: u64 = 31_536_000;

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
    ensure!(
        msg.recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {}
    );

    let current_time = env.block.time.seconds();
    let splitter = match msg.lock_time {
        Some(lock_time) => {
            // New lock time can't be too short
            ensure!(lock_time >= ONE_DAY, ContractError::LockTimeTooShort {});

            // New lock time can't be too long
            ensure!(lock_time <= ONE_YEAR, ContractError::LockTimeTooLong {});

            Splitter {
                recipients: msg.recipients,
                lock: Expiration::AtTime(Timestamp::from_seconds(lock_time + current_time)),
            }
        }
        None => {
            Splitter {
                recipients: msg.recipients,
                // If locking isn't desired upon instantiation, it's automatically set to 0
                lock: Expiration::AtTime(Timestamp::from_seconds(current_time)),
            }
        }
    };
    // Save kernel address after validating it

    SPLITTER.save(deps.storage, &splitter)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "splitter".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: msg.modules,
            kernel_address: msg.kernel_address,
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

    // Do this before the hooks get fired off to ensure that there are no errors from the app
    // address not being fully setup yet.
    if let ExecuteMsg::AndrReceive(andr_msg) = msg.clone() {
        if let AndromedaMsg::UpdateAppContract { address } = andr_msg {
            let splitter = SPLITTER.load(deps.storage)?;
            let mut addresses: Vec<String> = vec![];
            for splitter_recipient in splitter.recipients {
                addresses.push(splitter_recipient.recipient.address);
            }
            return contract.execute_update_app_contract(deps, info, address, Some(addresses));
        } else if let AndromedaMsg::UpdateOwner { address } = andr_msg {
            return contract.execute_update_owner(deps, info, address);
        }
    }

    //Andromeda Messages can be executed without modules, if they are a wrapped execute message they will loop back
    if let ExecuteMsg::AndrReceive(andr_msg) = msg {
        return contract.execute(deps, env, info, andr_msg, execute);
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
        ExecuteMsg::AndrReceive(msg) => execute_andromeda(deps, env, info, msg),
        ExecuteMsg::AMPReceive(pkt) => handle_amp_packet(deps, env, info, pkt),
        ExecuteMsg::UpdateRecipients { recipients } => {
            execute_update_recipients(deps, env, info, recipients)
        }
        ExecuteMsg::UpdateLock { lock_time } => execute_update_lock(deps, env, info, lock_time),
        ExecuteMsg::Send { reply_gas, packet } => execute_send(deps, env, info, reply_gas, packet),
    }
}

pub struct ExecuteEnv<'a> {
    deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}

fn handle_amp_packet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    packet: AMPPkt,
) -> Result<Response, ContractError> {
    let mut res = Response::default();

    // Get kernel address
    let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;

    // Original packet sender
    let origin = packet.get_origin();

    // This contract will become the previous sender after sending the message back to the kernel
    let previous_sender = env.clone().contract.address;

    let execute_env = ExecuteEnv { deps, env, info };

    let msg_opt = packet.messages.first();

    if let Some(msg) = msg_opt {
        let exec_msg: ExecuteMsg = from_binary(&msg.message)?;
        let funds = msg.funds.to_vec();
        let mut exec_info = execute_env.info.clone();
        exec_info.funds = funds.clone();

        if msg.exit_at_error {
            let env = execute_env.env.clone();
            let mut exec_res = execute(execute_env.deps, env, exec_info, exec_msg)?;

            if packet.messages.len() > 1 {
                let adjusted_messages: Vec<AMPMsg> =
                    packet.messages.iter().skip(1).cloned().collect();

                let unused_funds: Vec<Coin> = adjusted_messages
                    .iter()
                    .flat_map(|msg| msg.funds.iter().cloned())
                    .collect();

                let kernel_message = generate_msg_native_kernel(
                    unused_funds,
                    origin,
                    previous_sender.to_string(),
                    adjusted_messages,
                    kernel_address.into_string(),
                )?;

                exec_res.messages.push(kernel_message);
            }

            res = res
                .add_attributes(exec_res.attributes)
                .add_submessages(exec_res.messages)
                .add_events(exec_res.events);
        } else {
            match execute(
                execute_env.deps,
                execute_env.env.clone(),
                exec_info,
                exec_msg,
            ) {
                Ok(mut exec_res) => {
                    if packet.messages.len() > 1 {
                        let adjusted_messages: Vec<AMPMsg> =
                            packet.messages.iter().skip(1).cloned().collect();

                        let unused_funds: Vec<Coin> = adjusted_messages
                            .iter()
                            .flat_map(|msg| msg.funds.iter().cloned())
                            .collect();

                        let kernel_message = generate_msg_native_kernel(
                            unused_funds,
                            origin,
                            previous_sender.to_string(),
                            adjusted_messages,
                            kernel_address.into_string(),
                        )?;

                        exec_res.messages.push(kernel_message);
                    }

                    res = res
                        .add_attributes(exec_res.attributes)
                        .add_submessages(exec_res.messages)
                        .add_events(exec_res.events);
                }
                Err(_) => {
                    // There's an error, but the user opted for the operation to proceed
                    // No funds are used in the event of an error
                    if packet.messages.len() > 1 {
                        let adjusted_messages: Vec<AMPMsg> =
                            packet.messages.iter().skip(1).cloned().collect();

                        let kernel_message = generate_msg_native_kernel(
                            funds,
                            origin,
                            previous_sender.to_string(),
                            adjusted_messages,
                            kernel_address.into_string(),
                        )?;
                        res = res.add_submessage(kernel_message);
                    }
                }
            }
        }
    }

    Ok(res)
}

fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    reply_gas: ReplyGasExit,
    packet: Option<AMPPkt>,
) -> Result<Response, ContractError> {
    let sent_funds: Vec<Coin> = info.funds.clone();
    ensure!(
        !sent_funds.is_empty(),
        ContractError::InvalidFunds {
            msg: "At least one coin should to be sent".to_string(),
        }
    );
    for coin in sent_funds.clone() {
        ensure!(
            !coin.amount.is_zero(),
            ContractError::InvalidFunds {
                msg: "Amount must be non-zero".to_string(),
            }
        );
    }

    let splitter = SPLITTER.load(deps.storage)?;

    let mut msgs: Vec<SubMsg> = Vec::new();
    let mut amp_msgs: Vec<AMPMsg> = Vec::new();
    let mut kernel_funds: Vec<Coin> = Vec::new();

    let mut remainder_funds = info.funds.clone();
    // Looking at this nested for loop, we could find a way to reduce time/memory complexity to avoid DoS.
    // Would like to understand more about why we loop through funds and what it exactly stored in it.
    // From there we could look into HashMaps, or other methods to break the nested loops and avoid Denial of Service.
    // [ACK-04] Limit number of coins sent to 5.
    ensure!(
        info.funds.len() < 5,
        ContractError::ExceedsMaxAllowedCoins {}
    );

    for splitter_recipient in &splitter.recipients {
        let recipient_percent = splitter_recipient.percent;
        let mut vec_coin: Vec<Coin> = Vec::new();
        for (i, coin) in sent_funds.iter().enumerate() {
            let mut recip_coin: Coin = coin.clone();
            recip_coin.amount = coin.amount * recipient_percent;
            remainder_funds[i].amount -= recip_coin.amount;
            vec_coin.push(recip_coin);
        }

        // ADO receivers must use AndromedaMsg::Receive to execute their functionality
        // Others may just receive the funds
        let recipient_addr = splitter_recipient.recipient.address.clone();

        match &splitter_recipient.recipient.msg {
            None => msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: splitter_recipient.recipient.address.clone(),
                amount: vec_coin,
            }))),

            Some(message) => {
                amp_msgs.push(AMPMsg::new(
                    recipient_addr,
                    message.clone(),
                    Some(vec_coin.clone()),
                    Some(reply_gas.reply_on.clone().unwrap_or(ReplyOn::Always)),
                    reply_gas.exit_at_error,
                    reply_gas.gas_limit,
                ));
                // Add the coins intended for the kernel
                for x in &vec_coin {
                    kernel_funds.push(x.to_owned())
                }
            }
        }
    }
    remainder_funds.retain(|x| x.amount > Uint128::zero());
    // Who is the sender of this function?

    // Why does the remaining funds go the the sender of the executor of the splitter?
    // Is it considered tax(fee) or mistake?
    // Discussion around caller of splitter function in andromedaSPLITTER smart contract.
    // From tests, it looks like owner of smart contract (Andromeda) will recieve the rest of funds.
    // If so, should be documented
    if !remainder_funds.is_empty() {
        msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: remainder_funds,
        })));
    }

    // Generates the SubMsg intended for the kernel
    // Check if any messages are intended for kernel in the first place
    let contract = ADOContract::default();

    // The original sender of the message
    let origin = match packet {
        Some(p) => p.get_origin(),
        None => info.sender.to_string(),
    };

    // The previous sender of the message is the contract
    let previous_sender = env.contract.address;

    if !amp_msgs.is_empty() {
        // The kernel address has been validated and saved during instantiation
        let kernel_address = contract.get_kernel_address(deps.storage)?;

        let msg = generate_msg_native_kernel(
            kernel_funds,
            origin,
            previous_sender.into_string(),
            amp_msgs,
            kernel_address.into_string(),
        )?;
        msgs.push(msg);
    }

    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "send")
        .add_attribute("sender", info.sender.to_string()))
}

pub fn execute_andromeda(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(binary) => {
            let (reply_gas, packet) = if let Some(rep_gas_pkt) = binary {
                let reply_gas_packet: (ReplyGasExit, Option<AMPPkt>) = from_binary(&rep_gas_pkt)?;
                reply_gas_packet
            } else {
                (
                    ReplyGasExit {
                        reply_on: None,
                        gas_limit: None,
                        exit_at_error: Some(true),
                    },
                    None,
                )
            };
            execute_send(deps, env, info, reply_gas, packet)
        }
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

fn execute_update_recipients(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipients: Vec<AddressPercent>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    validate_recipient_list(recipients.clone())?;

    let mut splitter = SPLITTER.load(deps.storage)?;
    // Can't call this function while the lock isn't expired

    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );
    // Max 100 recipients
    ensure!(
        recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {}
    );

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
    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut splitter = SPLITTER.load(deps.storage)?;

    // Can't call this function while the lock isn't expired

    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );
    // Get current time
    let current_time = env.block.time.seconds();

    // New lock time can't be too short
    ensure!(lock_time >= ONE_DAY, ContractError::LockTimeTooShort {});

    // New lock time can't be unreasonably long
    ensure!(lock_time <= ONE_YEAR, ContractError::LockTimeTooLong {});

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
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

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
    use andromeda_os::kernel::ExecuteMsg as KernelExecuteMsg;
    use andromeda_os::messages::AMPPkt;
    use andromeda_os::messages::ExecuteMsg::AMPReceive as AMPExecuteMsg;
    use andromeda_os::recipient::Recipient;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, to_binary, Coin, Decimal, WasmMsg};

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
            lock_time: Some(100_000),
            kernel_address: Some("kernel_address".to_string()),
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
                env.clone(),
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    ado_version: CONTRACT_VERSION.to_string(),
                    operators: None,
                    modules: None,
                    kernel_address: None,
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
                env.clone(),
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    ado_version: CONTRACT_VERSION.to_string(),
                    operators: None,
                    modules: None,
                    kernel_address: None,
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
        let msg = ExecuteMsg::Send {
            reply_gas: ReplyGasExit {
                reply_on: None,
                gas_limit: None,
                exit_at_error: Some(true),
            },
            packet: None,
        };

        let splitter = Splitter {
            recipients: recipient,
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    ado_version: CONTRACT_VERSION.to_string(),
                    operators: None,
                    modules: None,
                    kernel_address: Some("kernel".to_string()),
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
    fn test_execute_send_ado_recipient() {
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
        let msg = ExecuteMsg::Send {
            reply_gas: ReplyGasExit {
                reply_on: None,
                gas_limit: None,
                exit_at_error: Some(true),
            },
            packet: None,
        };

        let splitter = Splitter {
            recipients: recipient,
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    ado_version: CONTRACT_VERSION.to_string(),
                    operators: None,
                    modules: None,
                    kernel_address: Some("kernel".to_string()),
                },
            )
            .unwrap();

        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

        let pkt = AMPPkt::new(
            info.sender,
            "cosmos2contract",
            vec![
                AMPMsg::new(
                    recip_address1,
                    Binary::default(),
                    Some(vec![Coin::new(1000, "uluna")]),
                    None,
                    None,
                    None,
                ),
                AMPMsg::new(
                    recip_address2,
                    Binary::default(),
                    Some(vec![Coin::new(2000, "uluna")]),
                    None,
                    None,
                    None,
                ),
            ],
        );

        let expected_res = Response::new()
            .add_submessages(vec![
                SubMsg::new(
                    // refunds remainder to sender
                    CosmosMsg::Bank(BankMsg::Send {
                        to_address: owner.to_string(),
                        amount: vec![Coin::new(7000, "uluna")], // 10000 * 0.7   remainder
                    }),
                ),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: "kernel".to_string(),
                    msg: to_binary(&KernelExecuteMsg::AMPReceive(pkt)).unwrap(),
                    funds: vec![Coin::new(1000, "uluna"), Coin::new(2000, "uluna")],
                }),
            ])
            .add_attributes(vec![attr("action", "send"), attr("sender", "creator")]);

        assert_eq!(res, expected_res);
    }
    // testinn

    #[test]
    fn test_handle_packet_exit_with_error_true() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let sender_funds_amount = 0u128;
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
        let pkt = AMPPkt::new(
            info.clone().sender,
            "cosmos2contract",
            vec![
                AMPMsg::new(
                    recip_address1,
                    to_binary(&ExecuteMsg::Send {
                        reply_gas: ReplyGasExit {
                            reply_on: None,
                            gas_limit: None,
                            exit_at_error: Some(true),
                        },
                        packet: None,
                    })
                    .unwrap(),
                    Some(vec![Coin::new(0, "uluna")]),
                    None,
                    Some(true),
                    None,
                ),
                AMPMsg::new(
                    recip_address2,
                    to_binary(&ExecuteMsg::Send {
                        reply_gas: ReplyGasExit {
                            reply_on: None,
                            gas_limit: None,
                            exit_at_error: Some(true),
                        },
                        packet: None,
                    })
                    .unwrap(),
                    Some(vec![Coin::new(0, "uluna")]),
                    None,
                    Some(true),
                    None,
                ),
            ],
        );
        let msg = ExecuteMsg::AMPReceive(pkt);

        let splitter = Splitter {
            recipients: recipient,
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    ado_version: CONTRACT_VERSION.to_string(),
                    operators: None,
                    modules: None,
                    kernel_address: Some("kernel".to_string()),
                },
            )
            .unwrap();

        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

        assert_eq!(
            err,
            ContractError::InvalidFunds {
                msg: "Amount must be non-zero".to_string(),
            }
        );
    }

    #[test]
    fn test_execute_send_ado_recipient_exit_with_error_false() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let sender_funds_amount = 0u128;
        let owner = "creator";
        let info = mock_info(owner, &[Coin::new(sender_funds_amount, "uluna")]);

        let recip_address1 = "address1".to_string();
        let recip_percent1 = 10; // 10%

        let recip_address2 = "address2".to_string();
        let recip_percent2 = 20; // 20%

        let pkt = AMPPkt::new(
            info.clone().sender,
            "cosmos2contract",
            vec![
                AMPMsg::new(
                    recip_address1.clone(),
                    to_binary(&ExecuteMsg::Send {
                        reply_gas: ReplyGasExit {
                            reply_on: None,
                            gas_limit: None,
                            exit_at_error: Some(false),
                        },
                        packet: None,
                    })
                    .unwrap(),
                    Some(vec![Coin::new(0, "uluna")]),
                    None,
                    Some(false),
                    None,
                ),
                AMPMsg::new(
                    recip_address2.clone(),
                    to_binary(&ExecuteMsg::Send {
                        reply_gas: ReplyGasExit {
                            reply_on: None,
                            gas_limit: None,
                            exit_at_error: Some(false),
                        },
                        packet: None,
                    })
                    .unwrap(),
                    Some(vec![Coin::new(0, "uluna")]),
                    None,
                    Some(false),
                    None,
                ),
            ],
        );
        let msg = ExecuteMsg::AMPReceive(pkt);

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

        let splitter = Splitter {
            recipients: recipient,
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    ado_version: CONTRACT_VERSION.to_string(),
                    operators: None,
                    modules: None,
                    kernel_address: Some("kernel".to_string()),
                },
            )
            .unwrap();

        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

        let pkt = AMPPkt::new(
            info.sender,
            "cosmos2contract",
            vec![AMPMsg::new(
                recip_address2,
                to_binary(&ExecuteMsg::Send {
                    reply_gas: ReplyGasExit {
                        reply_on: None,
                        gas_limit: None,
                        exit_at_error: Some(false),
                    },
                    packet: None,
                })
                .unwrap(),
                Some(vec![Coin::new(0, "uluna")]),
                None,
                Some(false),
                None,
            )],
        );

        let expected_res = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "kernel".to_string(),
            msg: to_binary(&AMPExecuteMsg(pkt)).unwrap(),
            funds: coins(0, "uluna"),
        }));

        assert_eq!(res.messages[0], expected_res);
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
        let msg = ExecuteMsg::Send {
            reply_gas: ReplyGasExit {
                reply_on: None,
                gas_limit: None,
                exit_at_error: Some(true),
            },
            packet: None,
        };

        let splitter = Splitter {
            recipients: recipient,
            lock: Expiration::AtTime(Timestamp::from_seconds(0)),
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let deps_mut = deps.as_mut();
        ADOContract::default()
            .instantiate(
                deps_mut.storage,
                mock_env(),
                deps_mut.api,
                mock_info(owner, &[]),
                BaseInstantiateMsg {
                    ado_type: "splitter".to_string(),
                    ado_version: CONTRACT_VERSION.to_string(),
                    operators: None,
                    modules: None,
                    kernel_address: None,
                },
            )
            .unwrap();

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

        let expected_res = ContractError::ExceedsMaxAllowedCoins {};

        assert_eq!(res, expected_res);
    }
}
