use crate::state::SPLITTER;

use ado_base::ADOContract;
use andromeda_finance::weighted_splitter::{
    AddressWeight, ExecuteMsg, GetSplitterConfigResponse, GetUserWeightResponse, InstantiateMsg,
    MigrateMsg, QueryMsg, Splitter,
};
use andromeda_os::{
    messages::{AMPMsg, AMPPkt, ReplyGasExit},
    recipient::{generate_msg_native_kernel, AMPRecipient as Recipient},
};
use common::{
    ado_base::{hooks::AndromedaHook, AndromedaMsg, InstantiateMsg as BaseInstantiateMsg},
    encode_binary,
    error::ContractError,
};

use cosmwasm_std::{
    attr, ensure, entry_point, from_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, SubMsg, Timestamp, Uint128,
};

use cw_utils::{nonpayable, Expiration};
use semver::Version;

use cw2::{get_contract_version, set_contract_version};
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-weighted-distribution-splitter";
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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let app_contract = ADOContract::default().get_app_contract(deps.storage)?;
    // Max 100 recipients
    ensure!(
        msg.recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {}
    );
    // Validate recipients
    for address_weight in &msg.recipients {
        address_weight
            .recipient
            .validate_address(deps.api, &deps.querier, app_contract.clone())?
    }
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

    SPLITTER.save(deps.storage, &splitter)?;

    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "weighted-distribution-splitter".to_string(),
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
            let mut andr_addresses: Vec<String> = vec![];
            for recipient in splitter.recipients {
                if let Recipient::ADO(ado_recipient) = recipient.recipient {
                    andr_addresses.push(ado_recipient.address);
                }
            }
            return contract.execute_update_app_contract(deps, info, address, Some(andr_addresses));
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
        ExecuteMsg::UpdateRecipientWeight { recipient } => {
            execute_update_recipient_weight(deps, env, info, recipient)
        }
        ExecuteMsg::AddRecipient { recipient } => execute_add_recipient(deps, env, info, recipient),
        ExecuteMsg::RemoveRecipient { recipient } => {
            execute_remove_recipient(deps, env, info, recipient)
        }
        ExecuteMsg::UpdateLock { lock_time } => execute_update_lock(deps, env, info, lock_time),

        ExecuteMsg::Send {
            reply_gas_exit,
            packet,
        } => execute_send(deps, env, info, reply_gas_exit, packet),
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

pub fn execute_update_recipient_weight(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: AddressWeight,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    // Only the contract's owner can update a recipient's weight
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    // Can't set weight to 0
    ensure!(
        recipient.weight > Uint128::zero(),
        ContractError::InvalidWeight {}
    );

    // Splitter's lock should be expired
    let mut splitter = SPLITTER.load(deps.storage)?;

    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );

    // Recipients are stored in a vector, we search for the desired recipient's index in the vector

    let user_index = splitter
        .recipients
        .clone()
        .into_iter()
        .position(|x| x.recipient == recipient.recipient);

    // If the index exists, change the element's weight.
    // If the index doesn't exist, the recipient isn't on the list
    ensure!(user_index.is_some(), ContractError::UserNotFound {});

    if let Some(i) = user_index {
        splitter.recipients[i].weight = recipient.weight;
        SPLITTER.save(deps.storage, &splitter)?;
    };
    Ok(Response::default().add_attribute("action", "updated_recipient_weight"))
}

pub fn execute_add_recipient(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: AddressWeight,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    // Only the contract's owner can add a recipient
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    // No need to send funds

    // Check if splitter is locked
    let mut splitter = SPLITTER.load(deps.storage)?;

    // Can't add recipients while the lock isn't expired

    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );

    // Can't set weight to 0
    ensure!(
        recipient.weight > Uint128::zero(),
        ContractError::InvalidWeight {}
    );

    // Check for duplicate recipients

    let user_exists = splitter
        .recipients
        .iter()
        .any(|x| x.recipient == recipient.recipient);

    ensure!(!user_exists, ContractError::DuplicateRecipient {});

    // Adding a recipient can't push the total number of recipients over 100

    ensure!(
        splitter.recipients.len() < 100,
        ContractError::ReachedRecipientLimit {}
    );

    splitter.recipients.push(recipient);
    let new_splitter = Splitter {
        recipients: splitter.recipients,
        lock: splitter.lock,
    };
    SPLITTER.save(deps.storage, &new_splitter)?;

    Ok(Response::default().add_attributes(vec![attr("action", "added_recipient")]))
}

pub fn execute_andromeda(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(binary) => {
            let (reply_gas_exit, packet) = if let Some(rep_gas_pkt) = binary {
                let reply_gas_packet: (Option<ReplyGasExit>, Option<AMPPkt>) =
                    from_binary(&rep_gas_pkt)?;
                reply_gas_packet
            } else {
                (None, None)
            };
            execute_send(deps, env, info, reply_gas_exit, packet)
        }
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    reply_gas_exit: Option<ReplyGasExit>,
    packet: Option<AMPPkt>,
) -> Result<Response, ContractError> {
    // Amount of coins sent should be at least 1
    ensure!(
        !&info.funds.is_empty(),
        ContractError::InvalidFunds {
            msg: "ensure! at least one coin to be sent".to_string(),
        }
    );
    // Can't send more than 5 types of coins
    ensure!(
        info.funds.len() < 5,
        ContractError::ExceedsMaxAllowedCoins {}
    );

    let splitter = SPLITTER.load(deps.storage)?;
    let mut msgs: Vec<SubMsg> = Vec::new();
    let mut amp_msgs: Vec<AMPMsg> = Vec::new();
    let mut kernel_funds: Vec<Coin> = Vec::new();
    let mut remainder_funds = info.funds.clone();
    let mut total_weight = Uint128::zero();

    // Calculate the total weight of all recipients
    for recipient_addr in &splitter.recipients {
        let recipient_weight = recipient_addr.weight;
        total_weight += recipient_weight;
    }

    // Each recipient recieves the funds * (the recipient's weight / total weight of all recipients)
    // The remaining funds go to the sender of the function
    for recipient_addr in &splitter.recipients {
        let recipient_weight = recipient_addr.weight;
        let mut vec_coin: Vec<Coin> = Vec::new();
        for (i, coin) in info.funds.iter().enumerate() {
            let mut recip_coin: Coin = coin.clone();
            recip_coin.amount = coin.amount.multiply_ratio(recipient_weight, total_weight);
            remainder_funds[i].amount -= recip_coin.amount;
            vec_coin.push(recip_coin);
        }
        // ADO receivers must use AndromedaMsg::Receive to execute their functionality
        // Others may just receive the funds
        let recipient = recipient_addr.recipient.get_addr()?;

        let message = recipient_addr.recipient.get_message()?.unwrap_or_default();

        match &recipient_addr.recipient {
            Recipient::Addr(addr) => msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: addr.clone(),
                amount: vec_coin,
            }))),

            Recipient::ADO(_) => {
                if let Some(ref reply_gas_exit) = reply_gas_exit {
                    amp_msgs.push(AMPMsg::new(
                        recipient,
                        message,
                        Some(vec_coin.clone()),
                        reply_gas_exit.clone().reply_on,
                        reply_gas_exit.exit_at_error,
                        reply_gas_exit.gas_limit,
                    ));
                } else {
                    amp_msgs.push(AMPMsg::new(
                        recipient,
                        message,
                        Some(vec_coin.clone()),
                        None,
                        None,
                        None,
                    ))
                };
                // Add the coins intended for the kernel
                for x in &vec_coin {
                    kernel_funds.push(x.to_owned())
                }
            }
        }
    }
    remainder_funds.retain(|x| x.amount > Uint128::zero());

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
        .add_attributes(vec![attr("action", "send"), attr("sender", info.sender)]))
}

fn execute_update_recipients(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipients: Vec<AddressWeight>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    // Only the owner can use this function
    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    // No need to send funds

    // Recipient list can't be empty
    ensure!(
        !recipients.is_empty(),
        ContractError::EmptyRecipientsList {}
    );

    let mut splitter = SPLITTER.load(deps.storage)?;

    // Can't update recipients while lock isn't expired
    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );

    // Maximum number of recipients is 100
    ensure!(
        recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {}
    );

    // A recipient's weight has to be greater than zero
    let zero_weight = recipients.iter().any(|x| x.weight == Uint128::zero());

    ensure!(!zero_weight, ContractError::InvalidWeight {});

    splitter.recipients = recipients;
    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default().add_attributes(vec![attr("action", "update_recipients")]))
}

fn execute_remove_recipient(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    ensure!(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );

    let mut splitter = SPLITTER.load(deps.storage)?;

    // Can't remove recipients while lock isn't expired

    ensure!(
        splitter.lock.is_expired(&env.block),
        ContractError::ContractLocked {}
    );

    // Recipients are stored in a vector, we search for the desired recipient's index in the vector

    let user_index = splitter
        .recipients
        .clone()
        .into_iter()
        .position(|x| x.recipient == recipient);

    // If the index exists, remove the element found in the index
    // If the index doesn't exist, return an error
    ensure!(user_index.is_some(), ContractError::UserNotFound {});

    if let Some(i) = user_index {
        splitter.recipients.swap_remove(i);
        let new_splitter = Splitter {
            recipients: splitter.recipients,
            lock: splitter.lock,
        };
        SPLITTER.save(deps.storage, &new_splitter)?;
    };

    Ok(Response::default().add_attributes(vec![attr("action", "removed_recipient")]))
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

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {err}"))
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

    let addrs = recipients.iter().find(|&x| x.recipient == user);

    // Calculate the total weight
    let mut total_weight = Uint128::zero();
    for recipient_addr in &recipients {
        let recipient_weight = recipient_addr.weight;
        total_weight += recipient_weight;
    }

    if let Some(i) = addrs {
        let weight = i.weight;
        Ok(GetUserWeightResponse {
            weight,
            total_weight,
        })
    } else {
        Ok(GetUserWeightResponse {
            weight: Uint128::zero(),
            total_weight,
        })
    }
}

fn query_splitter(deps: Deps) -> Result<GetSplitterConfigResponse, ContractError> {
    let splitter = SPLITTER.load(deps.storage)?;

    Ok(GetSplitterConfigResponse { config: splitter })
}
