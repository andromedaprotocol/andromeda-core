use crate::state::SPLITTER;
use andromeda_finance::splitter::{
    validate_recipient_list, AddressPercent, ExecuteMsg, GetSplitterConfigResponse, InstantiateMsg,
    MigrateMsg, QueryMsg, ReplyGasExit, Splitter,
};
use andromeda_std::amp::messages::AMPMsg;
use andromeda_std::amp::{messages::AMPPkt, recipient::Recipient};
use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg},
    common::encode_binary,
    error::{from_semver, ContractError},
};
use andromeda_std::{ado_contract::ADOContract, common::context::ExecuteContext};
use cosmwasm_std::{
    attr, ensure, entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, SubMsg, Timestamp, Uint128,
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

#[cfg_attr(not(feature = "library"), entry_point)]
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

    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: "crowdfund".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    let mod_resp =
        ADOContract::default().register_modules(info.sender.as_str(), deps.storage, msg.modules)?;

    Ok(inst_resp
        .add_attributes(mod_resp.attributes)
        .add_submessages(mod_resp.messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    // };

    contract.module_hook::<Response>(
        &deps.as_ref(),
        AndromedaHook::OnExecute {
            sender: info.sender.to_string(),
            payload: encode_binary(&msg)?,
        },
    )?;
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    // };

    contract.module_hook::<Response>(
        &ctx.deps.as_ref(),
        AndromedaHook::OnExecute {
            sender: ctx.info.sender.to_string(),
            payload: encode_binary(&msg)?,
        },
    )?;
    match msg {
        ExecuteMsg::UpdateRecipients { recipients } => execute_update_recipients(ctx, recipients),
        ExecuteMsg::UpdateLock { lock_time } => execute_update_lock(ctx, lock_time),
        ExecuteMsg::Send {} => execute_send(ctx.deps, ctx.env, ctx.info),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn execute_send(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
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

    let mut remainder_funds = info.funds.clone();
    // Looking at this nested for loop, we could find a way to reduce time/memory complexity to avoid DoS.
    // Would like to understand more about why we loop through funds and what it exactly stored in it.
    // From there we could look into HashMaps, or other methods to break the nested loops and avoid Denial of Service.
    // [ACK-04] Limit number of coins sent to 5.
    ensure!(
        info.funds.len() < 5,
        ContractError::ExceedsMaxAllowedCoins {}
    );

    for recipient_addr in &splitter.recipients {
        let recipient_percent = recipient_addr.percent;
        let mut vec_coin: Vec<Coin> = Vec::new();
        for (i, coin) in sent_funds.iter().enumerate() {
            let mut recip_coin: Coin = coin.clone();
            recip_coin.amount = coin.amount * recipient_percent;
            remainder_funds[i].amount -= recip_coin.amount;
            vec_coin.push(recip_coin);
        }

        let direct_message = recipient_addr
            .recipient
            .generate_direct_msg(&deps.as_ref(), vec_coin)?;
        msgs.push(direct_message);
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

    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "send")
        .add_attribute("sender", info.sender.to_string()))
}

fn execute_update_recipients(
    ctx: ExecuteContext,
    recipients: Vec<AddressPercent>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

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

fn execute_update_lock(ctx: ExecuteContext, lock_time: u64) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetSplitterConfig {} => encode_binary(&query_splitter(deps)?),
        _ => ADOContract::default().query::<QueryMsg>(deps, env, msg, None),
    }
}

fn query_splitter(deps: Deps) -> Result<GetSplitterConfigResponse, ContractError> {
    let splitter = SPLITTER.load(deps.storage)?;

    Ok(GetSplitterConfigResponse { config: splitter })
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coins, from_binary, to_binary, Coin, Decimal, WasmMsg};

//     #[test]
//     fn test_instantiate() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();
//         let info = mock_info("creator", &[]);
//         let msg = InstantiateMsg {
//             recipients: vec![AddressPercent {
//                 recipient: Recipient::from_string(String::from("Some Address")),
//                 percent: Decimal::one(),
//             }],
//             modules: None,
//             lock_time: Some(100_000),
//             kernel_address: Some("kernel_address".to_string()),
//         };
//         let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
//         assert_eq!(0, res.messages.len());
//     }

//     #[test]
//     fn test_execute_update_lock() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();

//         let current_time = env.block.time.seconds();
//         let lock_time = 100_000;

//         let owner = "creator";

//         // Start off with an expiration that's behind current time (expired)
//         let splitter = Splitter {
//             recipients: vec![],
//             lock: Expiration::AtTime(Timestamp::from_seconds(current_time - 1)),
//         };

//         SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//         let msg = ExecuteMsg::UpdateLock { lock_time };
//         let deps_mut = deps.as_mut();
//         ADOContract::default()
//             .instantiate(
//                 deps_mut.storage,
//                 env.clone(),
//                 deps_mut.api,
//                 mock_info(owner, &[]),
//                 BaseInstantiateMsg {
//                     ado_type: "splitter".to_string(),
//                     ado_version: CONTRACT_VERSION.to_string(),
//                     operators: None,
//                     modules: None,
//                     kernel_address: None,
//                 },
//             )
//             .unwrap();

//         let info = mock_info(owner, &[]);
//         let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

//         let new_lock = Expiration::AtTime(Timestamp::from_seconds(current_time + lock_time));
//         assert_eq!(
//             Response::default().add_attributes(vec![
//                 attr("action", "update_lock"),
//                 attr("locked", new_lock.to_string())
//             ]),
//             res
//         );

//         //check result
//         let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
//         assert!(!splitter.lock.is_expired(&env.block));
//         assert_eq!(new_lock, splitter.lock);
//     }

//     #[test]
//     fn test_execute_update_recipients() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();

//         let owner = "creator";

//         let recipient = vec![
//             AddressPercent {
//                 recipient: Recipient::from_string(String::from("addr1")),
//                 percent: Decimal::percent(40),
//             },
//             AddressPercent {
//                 recipient: Recipient::from_string(String::from("addr1")),
//                 percent: Decimal::percent(60),
//             },
//         ];
//         let msg = ExecuteMsg::UpdateRecipients {
//             recipients: recipient.clone(),
//         };

//         let splitter = Splitter {
//             recipients: vec![],
//             lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//         };

//         SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//         let deps_mut = deps.as_mut();
//         ADOContract::default()
//             .instantiate(
//                 deps_mut.storage,
//                 env.clone(),
//                 deps_mut.api,
//                 mock_info(owner, &[]),
//                 BaseInstantiateMsg {
//                     ado_type: "splitter".to_string(),
//                     ado_version: CONTRACT_VERSION.to_string(),
//                     operators: None,
//                     modules: None,
//                     kernel_address: None,
//                 },
//             )
//             .unwrap();

//         let info = mock_info("incorrect_owner", &[]);
//         let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
//         assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

//         let info = mock_info(owner, &[]);
//         let res = execute(deps.as_mut(), env, info, msg).unwrap();
//         assert_eq!(
//             Response::default().add_attributes(vec![attr("action", "update_recipients")]),
//             res
//         );

//         //check result
//         let splitter = SPLITTER.load(deps.as_ref().storage).unwrap();
//         assert_eq!(splitter.recipients, recipient);
//     }

//     #[test]
//     fn test_execute_send() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();

//         let sender_funds_amount = 10000u128;
//         let owner = "creator";
//         let info = mock_info(owner, &[Coin::new(sender_funds_amount, "uluna")]);

//         let recip_address1 = "address1".to_string();
//         let recip_percent1 = 10; // 10%

//         let recip_address2 = "address2".to_string();
//         let recip_percent2 = 20; // 20%

//         let recipient = vec![
//             AddressPercent {
//                 recipient: Recipient::from_string(recip_address1.clone()),
//                 percent: Decimal::percent(recip_percent1),
//             },
//             AddressPercent {
//                 recipient: Recipient::from_string(recip_address2.clone()),
//                 percent: Decimal::percent(recip_percent2),
//             },
//         ];
//         let msg = ExecuteMsg::Send {
//             reply_gas: ReplyGasExit {
//                 reply_on: None,
//                 gas_limit: None,
//                 exit_at_error: Some(true),
//             },
//             packet: None,
//         };

//         let splitter = Splitter {
//             recipients: recipient,
//             lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//         };

//         SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//         let deps_mut = deps.as_mut();
//         ADOContract::default()
//             .instantiate(
//                 deps_mut.storage,
//                 mock_env(),
//                 deps_mut.api,
//                 mock_info(owner, &[]),
//                 BaseInstantiateMsg {
//                     ado_type: "splitter".to_string(),
//                     ado_version: CONTRACT_VERSION.to_string(),
//                     operators: None,
//                     modules: None,
//                     kernel_address: Some("kernel".to_string()),
//                 },
//             )
//             .unwrap();

//         let res = execute(deps.as_mut(), env, info, msg).unwrap();

//         let expected_res = Response::new()
//             .add_submessages(vec![
//                 SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//                     to_address: recip_address1,
//                     amount: vec![Coin::new(1000, "uluna")], // 10000 * 0.1
//                 })),
//                 SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//                     to_address: recip_address2,
//                     amount: vec![Coin::new(2000, "uluna")], // 10000 * 0.2
//                 })),
//                 SubMsg::new(
//                     // refunds remainder to sender
//                     CosmosMsg::Bank(BankMsg::Send {
//                         to_address: owner.to_string(),
//                         amount: vec![Coin::new(7000, "uluna")], // 10000 * 0.7   remainder
//                     }),
//                 ),
//             ])
//             .add_attributes(vec![attr("action", "send"), attr("sender", "creator")]);

//         assert_eq!(res, expected_res);
//     }

//     #[test]
//     fn test_execute_send_ado_recipient() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();

//         let sender_funds_amount = 10000u128;
//         let owner = "creator";
//         let info = mock_info(owner, &[Coin::new(sender_funds_amount, "uluna")]);

//         let recip_address1 = "address1".to_string();
//         let recip_percent1 = 10; // 10%

//         let recip_address2 = "address2".to_string();
//         let recip_percent2 = 20; // 20%

//         let recipient = vec![
//             AddressPercent {
//                 recipient: Recipient::ADO(ADORecipient {
//                     address: recip_address1.clone(),
//                     msg: None,
//                 }),
//                 percent: Decimal::percent(recip_percent1),
//             },
//             AddressPercent {
//                 recipient: Recipient::ADO(ADORecipient {
//                     address: recip_address2.clone(),
//                     msg: None,
//                 }),
//                 percent: Decimal::percent(recip_percent2),
//             },
//         ];
//         let msg = ExecuteMsg::Send {
//             reply_gas: ReplyGasExit {
//                 reply_on: None,
//                 gas_limit: None,
//                 exit_at_error: Some(true),
//             },
//             packet: None,
//         };

//         let splitter = Splitter {
//             recipients: recipient,
//             lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//         };

//         SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//         let deps_mut = deps.as_mut();
//         ADOContract::default()
//             .instantiate(
//                 deps_mut.storage,
//                 mock_env(),
//                 deps_mut.api,
//                 mock_info(owner, &[]),
//                 BaseInstantiateMsg {
//                     ado_type: "splitter".to_string(),
//                     ado_version: CONTRACT_VERSION.to_string(),
//                     operators: None,
//                     modules: None,
//                     kernel_address: Some("kernel".to_string()),
//                 },
//             )
//             .unwrap();

//         let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

//         let pkt = AMPPkt::new(
//             info.sender,
//             "cosmos2contract",
//             vec![
//                 AMPMsg::new(
//                     recip_address1,
//                     Binary::default(),
//                     Some(vec![Coin::new(1000, "uluna")]),
//                     None,
//                     None,
//                     None,
//                 ),
//                 AMPMsg::new(
//                     recip_address2,
//                     Binary::default(),
//                     Some(vec![Coin::new(2000, "uluna")]),
//                     None,
//                     None,
//                     None,
//                 ),
//             ],
//         );

//         let expected_res = Response::new()
//             .add_submessages(vec![
//                 SubMsg::new(
//                     // refunds remainder to sender
//                     CosmosMsg::Bank(BankMsg::Send {
//                         to_address: owner.to_string(),
//                         amount: vec![Coin::new(7000, "uluna")], // 10000 * 0.7   remainder
//                     }),
//                 ),
//                 SubMsg::new(WasmMsg::Execute {
//                     contract_addr: "kernel".to_string(),
//                     msg: to_binary(&KernelExecuteMsg::AMPReceive(pkt)).unwrap(),
//                     funds: vec![Coin::new(1000, "uluna"), Coin::new(2000, "uluna")],
//                 }),
//             ])
//             .add_attributes(vec![attr("action", "send"), attr("sender", "creator")]);

//         assert_eq!(res, expected_res);
//     }
//     // testinn

//     #[test]
//     fn test_handle_packet_exit_with_error_true() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();

//         let sender_funds_amount = 0u128;
//         let owner = "creator";
//         let info = mock_info(owner, &[Coin::new(sender_funds_amount, "uluna")]);

//         let recip_address1 = "address1".to_string();
//         let recip_percent1 = 10; // 10%

//         let recip_address2 = "address2".to_string();
//         let recip_percent2 = 20; // 20%

//         let recipient = vec![
//             AddressPercent {
//                 recipient: Recipient::ADO(ADORecipient {
//                     address: recip_address1.clone(),
//                     msg: None,
//                 }),
//                 percent: Decimal::percent(recip_percent1),
//             },
//             AddressPercent {
//                 recipient: Recipient::ADO(ADORecipient {
//                     address: recip_address2.clone(),
//                     msg: None,
//                 }),
//                 percent: Decimal::percent(recip_percent2),
//             },
//         ];
//         let pkt = AMPPkt::new(
//             info.clone().sender,
//             "cosmos2contract",
//             vec![
//                 AMPMsg::new(
//                     recip_address1,
//                     to_binary(&ExecuteMsg::Send {
//                         reply_gas: ReplyGasExit {
//                             reply_on: None,
//                             gas_limit: None,
//                             exit_at_error: Some(true),
//                         },
//                         packet: None,
//                     })
//                     .unwrap(),
//                     Some(vec![Coin::new(0, "uluna")]),
//                     None,
//                     Some(true),
//                     None,
//                 ),
//                 AMPMsg::new(
//                     recip_address2,
//                     to_binary(&ExecuteMsg::Send {
//                         reply_gas: ReplyGasExit {
//                             reply_on: None,
//                             gas_limit: None,
//                             exit_at_error: Some(true),
//                         },
//                         packet: None,
//                     })
//                     .unwrap(),
//                     Some(vec![Coin::new(0, "uluna")]),
//                     None,
//                     Some(true),
//                     None,
//                 ),
//             ],
//         );
//         let msg = ExecuteMsg::AMPReceive(pkt);

//         let splitter = Splitter {
//             recipients: recipient,
//             lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//         };

//         SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//         let deps_mut = deps.as_mut();
//         ADOContract::default()
//             .instantiate(
//                 deps_mut.storage,
//                 mock_env(),
//                 deps_mut.api,
//                 mock_info(owner, &[]),
//                 BaseInstantiateMsg {
//                     ado_type: "splitter".to_string(),
//                     ado_version: CONTRACT_VERSION.to_string(),
//                     operators: None,
//                     modules: None,
//                     kernel_address: Some("kernel".to_string()),
//                 },
//             )
//             .unwrap();

//         let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

//         assert_eq!(
//             err,
//             ContractError::InvalidFunds {
//                 msg: "Amount must be non-zero".to_string(),
//             }
//         );
//     }

//     #[test]
//     fn test_execute_send_ado_recipient_exit_with_error_false() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();

//         let sender_funds_amount = 0u128;
//         let owner = "creator";
//         let info = mock_info(owner, &[Coin::new(sender_funds_amount, "uluna")]);

//         let recip_address1 = "address1".to_string();
//         let recip_percent1 = 10; // 10%

//         let recip_address2 = "address2".to_string();
//         let recip_percent2 = 20; // 20%

//         let pkt = AMPPkt::new(
//             info.clone().sender,
//             "cosmos2contract",
//             vec![
//                 AMPMsg::new(
//                     recip_address1.clone(),
//                     to_binary(&ExecuteMsg::Send {
//                         reply_gas: ReplyGasExit {
//                             reply_on: None,
//                             gas_limit: None,
//                             exit_at_error: Some(false),
//                         },
//                         packet: None,
//                     })
//                     .unwrap(),
//                     Some(vec![Coin::new(0, "uluna")]),
//                     None,
//                     Some(false),
//                     None,
//                 ),
//                 AMPMsg::new(
//                     recip_address2.clone(),
//                     to_binary(&ExecuteMsg::Send {
//                         reply_gas: ReplyGasExit {
//                             reply_on: None,
//                             gas_limit: None,
//                             exit_at_error: Some(false),
//                         },
//                         packet: None,
//                     })
//                     .unwrap(),
//                     Some(vec![Coin::new(0, "uluna")]),
//                     None,
//                     Some(false),
//                     None,
//                 ),
//             ],
//         );
//         let msg = ExecuteMsg::AMPReceive(pkt);

//         let recipient = vec![
//             AddressPercent {
//                 recipient: Recipient::ADO(ADORecipient {
//                     address: recip_address1,
//                     msg: None,
//                 }),
//                 percent: Decimal::percent(recip_percent1),
//             },
//             AddressPercent {
//                 recipient: Recipient::ADO(ADORecipient {
//                     address: recip_address2.clone(),
//                     msg: None,
//                 }),
//                 percent: Decimal::percent(recip_percent2),
//             },
//         ];

//         let splitter = Splitter {
//             recipients: recipient,
//             lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//         };

//         SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//         let deps_mut = deps.as_mut();
//         ADOContract::default()
//             .instantiate(
//                 deps_mut.storage,
//                 mock_env(),
//                 deps_mut.api,
//                 mock_info(owner, &[]),
//                 BaseInstantiateMsg {
//                     ado_type: "splitter".to_string(),
//                     ado_version: CONTRACT_VERSION.to_string(),
//                     operators: None,
//                     modules: None,
//                     kernel_address: Some("kernel".to_string()),
//                 },
//             )
//             .unwrap();

//         let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

//         let pkt = AMPPkt::new(
//             info.sender,
//             "cosmos2contract",
//             vec![AMPMsg::new(
//                 recip_address2,
//                 to_binary(&ExecuteMsg::Send {
//                     reply_gas: ReplyGasExit {
//                         reply_on: None,
//                         gas_limit: None,
//                         exit_at_error: Some(false),
//                     },
//                     packet: None,
//                 })
//                 .unwrap(),
//                 Some(vec![Coin::new(0, "uluna")]),
//                 None,
//                 Some(false),
//                 None,
//             )],
//         );

//         let expected_res = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: "kernel".to_string(),
//             msg: to_binary(&AMPExecuteMsg(pkt)).unwrap(),
//             funds: coins(0, "uluna"),
//         }));

//         assert_eq!(res.messages[0], expected_res);
//     }

//     #[test]
//     fn test_query_splitter() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();
//         let splitter = Splitter {
//             recipients: vec![],
//             lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//         };

//         SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//         let query_msg = QueryMsg::GetSplitterConfig {};
//         let res = query(deps.as_ref(), env, query_msg).unwrap();
//         let val: GetSplitterConfigResponse = from_binary(&res).unwrap();

//         assert_eq!(val.config, splitter);
//     }

//     #[test]
//     fn test_execute_send_error() {
//         //Executes send with more than 5 tokens [ACK-04]
//         let mut deps = mock_dependencies();
//         let env = mock_env();

//         let sender_funds_amount = 10000u128;
//         let owner = "creator";
//         let info = mock_info(
//             owner,
//             &vec![
//                 Coin::new(sender_funds_amount, "uluna"),
//                 Coin::new(sender_funds_amount, "uluna"),
//                 Coin::new(sender_funds_amount, "uluna"),
//                 Coin::new(sender_funds_amount, "uluna"),
//                 Coin::new(sender_funds_amount, "uluna"),
//                 Coin::new(sender_funds_amount, "uluna"),
//             ],
//         );

//         let recip_address1 = "address1".to_string();
//         let recip_percent1 = 10; // 10%

//         let recip_address2 = "address2".to_string();
//         let recip_percent2 = 20; // 20%

//         let recipient = vec![
//             AddressPercent {
//                 recipient: Recipient::from_string(recip_address1),
//                 percent: Decimal::percent(recip_percent1),
//             },
//             AddressPercent {
//                 recipient: Recipient::from_string(recip_address2),
//                 percent: Decimal::percent(recip_percent2),
//             },
//         ];
//         let msg = ExecuteMsg::Send {
//             reply_gas: ReplyGasExit {
//                 reply_on: None,
//                 gas_limit: None,
//                 exit_at_error: Some(true),
//             },
//             packet: None,
//         };

//         let splitter = Splitter {
//             recipients: recipient,
//             lock: Expiration::AtTime(Timestamp::from_seconds(0)),
//         };

//         SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

//         let deps_mut = deps.as_mut();
//         ADOContract::default()
//             .instantiate(
//                 deps_mut.storage,
//                 mock_env(),
//                 deps_mut.api,
//                 mock_info(owner, &[]),
//                 BaseInstantiateMsg {
//                     ado_type: "splitter".to_string(),
//                     ado_version: CONTRACT_VERSION.to_string(),
//                     operators: None,
//                     modules: None,
//                     kernel_address: None,
//                 },
//             )
//             .unwrap();

//         let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

//         let expected_res = ContractError::ExceedsMaxAllowedCoins {};

//         assert_eq!(res, expected_res);
//     }
// }
