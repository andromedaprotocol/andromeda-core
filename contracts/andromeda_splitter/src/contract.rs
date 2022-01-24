use crate::state::SPLITTER;
use andromeda_protocol::{
    communication::encode_binary,
    communication::{parse_message, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    modules::{
        address_list::{on_address_list_reply, AddressListModule, REPLY_ADDRESS_LIST},
        hooks::{HookResponse, MessageHooks},
        Module, Modules,
    },
    operators::{execute_update_operators, query_is_operator, query_operators},
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    splitter::{
        validate_recipient_list, AddressPercent, ExecuteMsg, GetSplitterConfigResponse,
        InstantiateMsg, QueryMsg, Splitter,
    },
};
use cosmwasm_std::{
    attr, entry_point, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, SubMsg, Uint128,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate()?;

    let splitter = Splitter {
        recipients: msg.recipients,
        locked: false,
        address_list: msg.address_list.clone(),
    };

    // let inst_msgs = generate_instantiate_msgs(&deps, info.clone(), env, vec![msg.address_list])?;
    let mut module_defs = vec![];
    if msg.address_list.is_some() {
        module_defs.push(msg.address_list.unwrap().as_definition());
    }
    let modules = Modules::new(module_defs);
    let inst_msgs =
        modules.hook(|module| module.on_instantiate(&deps, info.clone(), env.clone()))?;

    SPLITTER.save(deps.storage, &splitter)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "instantiate"),
            attr("type", "splitter"),
        ])
        .add_submessages(inst_msgs.msgs)
        .add_events(inst_msgs.events))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let splitter = SPLITTER.load(deps.storage)?;

    // [GLOBAL-02] Changing is_some() + .unwrap() to if let Some()
    if let Some(addr_list) = splitter.address_list {
        addr_list.on_execute(&deps, info.clone(), env.clone())?;
    }

    match msg {
        ExecuteMsg::UpdateRecipients { recipients } => {
            execute_update_recipients(deps, info, recipients)
        }
        ExecuteMsg::UpdateLock { lock } => execute_update_lock(deps, info, lock),
        ExecuteMsg::UpdateAddressList { address_list } => {
            execute_update_address_list(deps, info, env, address_list)
        }
        ExecuteMsg::Send {} => execute_send(deps, info),
        ExecuteMsg::AndrReceive(msg) => execute_andromeda(deps, env, info, msg),
    }
}

pub fn execute_andromeda(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(..) => execute_send(deps, info),
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    match msg.id {
        REPLY_ADDRESS_LIST => on_address_list_reply(deps, msg),
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

fn execute_send(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let sent_funds: Vec<Coin> = info.funds.clone();
    require(!sent_funds.is_empty(), ContractError::EmptyFunds {})?;

    let splitter = SPLITTER.load(deps.storage)?;
    let mut submsg: Vec<SubMsg> = Vec::new();

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
            recip_coin.amount = coin.amount.multiply_ratio(recipient_percent, 100u128);
            remainder_funds[i].amount -= recip_coin.amount;
            vec_coin.push(recip_coin);
        }
        // ADO receivers must use AndromedaMsg::Receive to execute their functionality
        // Others may just receive the funds
        let msg = recipient_addr
            .recipient
            .generate_msg(&deps.as_ref(), vec_coin)?;
        submsg.push(msg);
    }
    remainder_funds = remainder_funds
        .into_iter()
        .filter(|x| x.amount > Uint128::from(0u128))
        .collect();
    // Who is the sender of this function?
    // Why does the remaining funds go the the sender of the executor of the splitter?
    // Is it considered tax(fee) or mistake?
    // Discussion around caller of splitter function in andromeda_splitter smart contract.
    // From tests, it looks like owner of smart contract (Andromeda) will recieve the rest of funds.
    // If so, should be documented
    if !remainder_funds.is_empty() {
        submsg.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: remainder_funds,
        })));
    }

    Ok(Response::new().add_submessages(submsg).add_attributes(vec![
        attr("action", "send"),
        attr("sender", info.sender.to_string()),
    ]))
}

fn execute_update_recipients(
    deps: DepsMut,
    info: MessageInfo,
    recipients: Vec<AddressPercent>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
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

fn execute_update_lock(
    deps: DepsMut,
    info: MessageInfo,
    lock: bool,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
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

fn execute_update_address_list(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    address_list: Option<AddressListModule>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;

    let mut splitter = SPLITTER.load(deps.storage)?;
    if splitter.locked {
        StdError::generic_err("The splitter is currently locked");
    }

    let mod_resp = match address_list.clone() {
        None => HookResponse::default(),
        Some(addr_list) => addr_list.on_instantiate(&deps, info, env)?,
    };
    splitter.address_list = address_list;

    SPLITTER.save(deps.storage, &splitter)?;

    Ok(Response::default()
        .add_submessages(mod_resp.msgs)
        .add_events(mod_resp.events)
        .add_attributes(vec![attr("action", "update_address_list")]))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetSplitterConfig {} => encode_binary(&query_splitter(deps)?),
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let received: QueryMsg = parse_message(data)?;
            match received {
                QueryMsg::AndrQuery(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => query(deps, env, received),
            }
        }
        AndromedaQuery::Owner {} => encode_binary(&query_contract_owner(deps)?),
        AndromedaQuery::Operators {} => encode_binary(&query_operators(deps)?),
        AndromedaQuery::IsOperator { address } => {
            encode_binary(&query_is_operator(deps, &address)?)
        }
    }
}

fn query_splitter(deps: Deps) -> Result<GetSplitterConfigResponse, ContractError> {
    let splitter = SPLITTER.load(deps.storage)?;
    let address_list_contract = match splitter.clone().address_list {
        Some(addr_list) => addr_list.get_contract_address(deps.storage),
        None => None,
    };

    Ok(GetSplitterConfigResponse {
        config: splitter,
        address_list_contract,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::{communication::Recipient, modules::address_list::AddressListModule};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Addr, Coin, Uint128};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            address_list: None,
            recipients: vec![AddressPercent {
                recipient: Recipient::from_string(String::from("Some Address")),
                percent: Uint128::from(100_u128),
            }],
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_update_lock() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let owner = "creator";
        let info = mock_info(owner, &[]);

        let lock = true;
        let msg = ExecuteMsg::UpdateLock { lock };

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("incorrect_owner"))
            .unwrap();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        if let Ok(..) = res {
            panic!()
        }

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner.to_string()))
            .unwrap();

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
            address_list: None,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

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
    fn test_execute_update_address_list() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let owner = "creator";

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner.to_string()))
            .unwrap();

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
            address_list: None,
        };
        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let address_list = AddressListModule {
            address: Some(String::from("terra1contractaddress")),
            code_id: Some(1),
            operators: Some(vec![String::from("operator1")]),
            inclusive: true,
        };
        let msg = ExecuteMsg::UpdateAddressList {
            address_list: Some(address_list.clone()),
        };

        let unauth_info = mock_info("anyone", &[]);
        let err_res = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
        assert_eq!(err_res, ContractError::Unauthorized {});

        let info = mock_info(owner, &[]);
        let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let mod_resp = address_list
            .on_instantiate(&deps.as_mut(), info, env)
            .unwrap();
        let expected = Response::default()
            .add_submessages(mod_resp.msgs)
            .add_events(mod_resp.events)
            .add_attributes(vec![attr("action", "update_address_list")]);

        assert_eq!(resp, expected);

        let updated = SPLITTER.load(deps.as_mut().storage).unwrap();

        assert_eq!(updated.address_list.unwrap(), address_list);
    }

    #[test]
    fn test_execute_update_recipients() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let owner = "creator";
        let info = mock_info(owner, &[]);

        let recipient = vec![
            AddressPercent {
                recipient: Recipient::from_string(String::from("addr1")),
                percent: Uint128::from(40_u128),
            },
            AddressPercent {
                recipient: Recipient::from_string(String::from("addr1")),
                percent: Uint128::from(60_u128),
            },
        ];
        let msg = ExecuteMsg::UpdateRecipients {
            recipients: recipient.clone(),
        };

        //incorrect owner
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("incorrect_owner"))
            .unwrap();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        if let Ok(..) = res {
            panic!();
        }

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner.to_string()))
            .unwrap();

        let splitter = Splitter {
            recipients: vec![],
            locked: false,
            address_list: None,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

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
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let sender_funds_amount = 10000u128;
        let owner = "creator";
        let info = mock_info(owner, &[Coin::new(sender_funds_amount, "uluna")]);

        let recip_address1 = "address1".to_string();
        let recip_percent1 = 10u128; // 10%

        let recip_address2 = "address2".to_string();
        let recip_percent2 = 20u128; // 20%

        let recipient = vec![
            AddressPercent {
                recipient: Recipient::from_string(recip_address1.clone()),
                percent: Uint128::from(recip_percent1),
            },
            AddressPercent {
                recipient: Recipient::from_string(recip_address2.clone()),
                percent: Uint128::from(recip_percent2),
            },
        ];
        let msg = ExecuteMsg::Send {};

        //incorrect owner
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("incorrect_owner"))
            .unwrap();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        if let Ok(..) = res {
            panic!();
        }

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner.to_string()))
            .unwrap();

        let splitter = Splitter {
            recipients: recipient,
            locked: false,
            address_list: None,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

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
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let splitter = Splitter {
            recipients: vec![],
            locked: false,
            address_list: Some(AddressListModule {
                address: Some(String::from("somecontractaddress")),
                code_id: None,
                operators: None,
                inclusive: false,
            }),
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let query_msg = QueryMsg::GetSplitterConfig {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: GetSplitterConfigResponse = from_binary(&res).unwrap();

        assert_eq!(val.config, splitter);
        assert_eq!(
            val.address_list_contract.unwrap(),
            splitter.address_list.unwrap().address.unwrap()
        );
    }

    #[test]
    fn test_execute_send_error() {
        //Executes send with more than 5 tokens [ACK-04]
        let mut deps = mock_dependencies(&[]);
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
        let recip_percent1 = 10u128; // 10%

        let recip_address2 = "address2".to_string();
        let recip_percent2 = 20u128; // 20%

        let recipient = vec![
            AddressPercent {
                recipient: Recipient::from_string(recip_address1),
                percent: Uint128::from(recip_percent1),
            },
            AddressPercent {
                recipient: Recipient::from_string(recip_address2),
                percent: Uint128::from(recip_percent2),
            },
        ];
        let msg = ExecuteMsg::Send {};

        //incorrect owner
        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked("incorrect_owner"))
            .unwrap();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        if let Ok(..) = res {
            panic!()
        }

        CONTRACT_OWNER
            .save(deps.as_mut().storage, &Addr::unchecked(owner))
            .unwrap();

        let splitter = Splitter {
            recipients: recipient,
            locked: false,
            address_list: None,
        };

        SPLITTER.save(deps.as_mut().storage, &splitter).unwrap();

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();

        let expected_res = ContractError::ExceedsMaxAllowedCoins {};

        assert_eq!(res, expected_res);
    }
}
