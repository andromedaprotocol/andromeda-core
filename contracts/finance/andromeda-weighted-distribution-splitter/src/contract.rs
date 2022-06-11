use crate::state::SPLITTER;

use ado_base::ADOContract;
use andromeda_finance::weighted_splitter::{
    AddressWeight, ExecuteMsg, GetSplitterConfigResponse, GetUserWeightResponse, InstantiateMsg,
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
    SubMsg, Uint128,
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
    require(
        !msg.recipients.is_empty(),
        ContractError::EmptyRecipientsList {},
    )?;
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
        ExecuteMsg::UpdateRecipientWeight { recipient } => {
            execute_update_recipient_weight(deps, info, recipient)
        }
        ExecuteMsg::AddRecipient { recipient } => execute_add_recipient(deps, info, recipient),
        ExecuteMsg::RemoveRecipient { recipient } => {
            execute_remove_recipient(deps, info, recipient)
        }
        ExecuteMsg::UpdateLock { lock } => execute_update_lock(deps, info, lock),

        ExecuteMsg::Send {} => execute_send(deps, info),
        ExecuteMsg::AndrReceive(msg) => execute_andromeda(deps, env, info, msg),
    }
}

pub fn execute_update_recipient_weight(
    deps: DepsMut,
    info: MessageInfo,
    recipient: AddressWeight,
) -> Result<Response, ContractError> {
    // Only the contract's owner can update a recipient's weight
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // No need to send funds
    require(
        info.funds.is_empty(),
        ContractError::FunctionDeclinesFunds {},
    )?;
    // Can't set weight to 0
    require(
        recipient.weight > Uint128::zero(),
        ContractError::InvalidWeight {},
    )?;

    // Check if splitter is locked
    let mut splitter = SPLITTER.load(deps.storage)?;

    require(!splitter.locked, ContractError::ContractLocked {})?;

    // Recipients are stored in a vector, we search for the desired recipient's index in the vector

    let user_index = splitter
        .recipients
        .clone()
        .into_iter()
        .position(|x| x.recipient == recipient.recipient);

    // If the index exists, change the element's weight.
    // If the index doesn't exist, the recipient isn't on the list
    require(user_index.is_some(), ContractError::UserNotFound {})?;

    if let Some(i) = user_index {
        splitter.recipients[i].weight = recipient.weight;
        SPLITTER.save(deps.storage, &splitter)?;
    };
    Ok(Response::default().add_attribute("action", "updated_recipient_weight"))
}

pub fn execute_add_recipient(
    deps: DepsMut,
    info: MessageInfo,
    recipient: AddressWeight,
) -> Result<Response, ContractError> {
    // Only the contract's owner can add a recipient
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // No need to send funds
    require(
        info.funds.is_empty(),
        ContractError::FunctionDeclinesFunds {},
    )?;
    // Check if splitter is locked
    let mut splitter = SPLITTER.load(deps.storage)?;

    require(!splitter.locked, ContractError::ContractLocked {})?;

    // Can't set weight to 0
    require(
        recipient.weight > Uint128::zero(),
        ContractError::InvalidWeight {},
    )?;

    // Check for duplicate recipients

    let user_exists = splitter
        .recipients
        .iter()
        .any(|x| x.recipient == recipient.recipient);

    require(!user_exists, ContractError::DuplicateRecipient {})?;

    // Adding a recipient can't push the total number of recipients over 100

    require(
        splitter.recipients.len() < 100,
        ContractError::ReachedRecipientLimit {},
    )?;

    splitter.recipients.push(recipient);
    let new_splitter = Splitter {
        recipients: splitter.recipients,
        locked: splitter.locked,
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
        AndromedaMsg::Receive(..) => execute_send(deps, info),
        _ => ADOContract::default().execute(deps, env, info, msg, execute),
    }
}

fn execute_send(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Amount of coins sent should be at least 1
    require(
        !&info.funds.is_empty(),
        ContractError::InvalidFunds {
            msg: "Require at least one coin to be sent".to_string(),
        },
    )?;
    // Can't send more than 5 types of coins
    require(
        info.funds.len() < 5,
        ContractError::ExceedsMaxAllowedCoins {},
    )?;

    let splitter = SPLITTER.load(deps.storage)?;
    let mut msgs: Vec<SubMsg> = Vec::new();

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
    // Only the owner can use this function
    require(
        ADOContract::default().is_owner_or_operator(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    // No need to send funds
    require(
        info.funds.is_empty(),
        ContractError::FunctionDeclinesFunds {},
    )?;
    // Recipient list can't be empty
    require(
        !recipients.is_empty(),
        ContractError::EmptyRecipientsList {},
    )?;

    let mut splitter = SPLITTER.load(deps.storage)?;

    // Can't change splitter while locked
    require(!splitter.locked, ContractError::ContractLocked {})?;

    // Maximum number of recipients is 100
    require(
        recipients.len() <= 100,
        ContractError::ReachedRecipientLimit {},
    )?;

    // A recipient's weight has to be greater than zero
    let zero_weight = recipients.iter().any(|x| x.weight == Uint128::zero());

    require(!zero_weight, ContractError::InvalidWeight {})?;

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

    // No need to send funds
    require(
        info.funds.is_empty(),
        ContractError::FunctionDeclinesFunds {},
    )?;

    let mut splitter = SPLITTER.load(deps.storage)?;

    require(!splitter.locked, ContractError::ContractLocked {})?;

    // Recipients are stored in a vector, we search for the desired recipient's index in the vector

    let user_index = splitter
        .recipients
        .clone()
        .into_iter()
        .position(|x| x.recipient == recipient);

    // If the index exists, remove the element found in the index
    // If the index doesn't exist, return an error
    require(user_index.is_some(), ContractError::UserNotFound {})?;

    if let Some(i) = user_index {
        splitter.recipients.swap_remove(i);
        let new_splitter = Splitter {
            recipients: splitter.recipients,
            locked: splitter.locked,
        };
        SPLITTER.save(deps.storage, &new_splitter)?;
    };

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

    // No need to send funds
    require(
        info.funds.is_empty(),
        ContractError::FunctionDeclinesFunds {},
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
