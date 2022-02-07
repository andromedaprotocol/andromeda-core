#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use andromeda_protocol::{
    communication::{encode_binary, parse_message, AndromedaMsg, AndromedaQuery},
    error::ContractError,
    modules::{
        address_list::{on_address_list_reply, REPLY_ADDRESS_LIST},
        auction::{on_auction_reply, AUCTION_CONTRACT, REPLY_AUCTION},
        read_modules,
        receipt::{on_receipt_reply, REPLY_RECEIPT},
        store_modules, Modules,
    },
    operators::{execute_update_operators, query_is_operator, query_operators},
    ownership::{execute_update_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    token::{
        Approval, ExecuteMsg, InstantiateMsg, MigrateMsg, MintMsg, ModuleContract,
        ModuleInfoResponse, NftInfoResponseExtension, QueryMsg, Token, TokensResponse,
        TransferAgreement,
    },
};
use cosmwasm_std::{
    attr, coin, Addr, Api, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Pair, Reply,
    Response, StdError, StdResult,
};
use cw721::{
    AllNftInfoResponse, ApprovedForAllResponse, ContractInfoResponse, Cw721ReceiveMsg, Expiration,
    NftInfoResponse, NumTokensResponse, OwnerOfResponse,
};
use cw_storage_plus::Bound;

use crate::state::{
    decrement_num_tokens, has_transfer_rights, increment_num_tokens, load_token, mint_token,
    tokens, TokenConfig, CONFIG, NUM_TOKENS, OPERATOR,
};

const DEFAULT_LIMIT: u32 = 10u32;
const MAX_LIMIT: u32 = 30u32;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    require(
        msg.name.len() > 3,
        ContractError::InvalidTokenNameLength {
            msg: "Name must be between 3 and 30 characters.".to_string(),
        },
    )?;
    msg.validate()?;
    let config = TokenConfig {
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        minter: msg.minter.to_string(),
    };

    let modules = Modules::new(msg.modules);
    let mod_res = modules.hook(|module| module.on_instantiate(&deps, info.clone(), env.clone()))?;

    CONFIG.save(deps.storage, &config)?;
    CONTRACT_OWNER.save(deps.storage, &deps.api.addr_validate(&msg.minter)?)?;
    store_modules(deps.storage, modules, &deps.querier)?;

    Ok(Response::new()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "instantiate"),
            attr("name", msg.name),
            attr("symbol", msg.symbol),
            attr("minter", msg.minter),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    match msg.id {
        REPLY_RECEIPT => on_receipt_reply(deps, msg),
        REPLY_ADDRESS_LIST => on_address_list_reply(deps, msg),
        REPLY_AUCTION => on_auction_reply(deps, msg),
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let modules = read_modules(deps.storage)?;
    modules.hook(|module| module.on_execute(&deps, info.clone(), env.clone()))?;

    match msg {
        ExecuteMsg::AndrReceive(msg) => execute_andr_receive(deps, env, info, msg),
        ExecuteMsg::Mint(msg) => execute_mint(deps, env, info, msg),
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => execute_transfer(deps, env, info, recipient, token_id),
        ExecuteMsg::SendNft {
            contract,
            token_id,
            msg,
        } => execute_send_nft(deps, env, info, contract, token_id, msg),
        ExecuteMsg::Approve {
            spender,
            expires,
            token_id,
        } => execute_approve(deps, env, info, token_id, spender, expires),
        ExecuteMsg::Revoke { spender, token_id } => {
            execute_revoke(deps, env, info, token_id, spender)
        }
        ExecuteMsg::ApproveAll { operator, expires } => {
            execute_approve_all(deps, env, info, operator, expires)
        }
        ExecuteMsg::RevokeAll { operator } => execute_revoke_all(deps, env, info, operator),
        ExecuteMsg::TransferAgreement {
            token_id,
            denom,
            amount,
            purchaser,
        } => execute_transfer_agreement(deps, env, info, token_id, purchaser, amount.u128(), denom),
        ExecuteMsg::Burn { token_id } => execute_burn(deps, env, info, token_id),
        ExecuteMsg::Archive { token_id } => execute_archive(deps, env, info, token_id),
        ExecuteMsg::UpdatePricing { token_id, price } => {
            execute_update_pricing(deps, env, info, token_id, price)
        }
    }
}

fn execute_andr_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AndromedaMsg,
) -> Result<Response, ContractError> {
    match msg {
        AndromedaMsg::Receive(data) => {
            let received: ExecuteMsg = parse_message(data)?;
            match received {
                ExecuteMsg::AndrReceive(..) => Err(ContractError::NestedAndromedaMsg {}),
                _ => execute(deps, env, info, received),
            }
        }
        AndromedaMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        AndromedaMsg::UpdateOperators { operators } => {
            execute_update_operators(deps, info, operators)
        }
        AndromedaMsg::Withdraw { .. } => Err(ContractError::UnsupportedOperation {}),
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Box<MintMsg>,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&msg.owner)?;
    let token = Token {
        token_id: msg.token_id.clone(),
        owner: msg.owner.clone(),
        description: msg.description.clone(),
        name: msg.name.clone(),
        approvals: vec![],
        transfer_agreement: None,
        metadata: msg.metadata.clone(),
        token_uri: msg.token_uri.clone(),
        archived: false,
        pricing: msg.pricing.clone(),
        publisher: info.sender.to_string(),
    };
    let config = CONFIG.load(deps.storage)?;
    require(info.sender == config.minter, ContractError::Unauthorized {})?;

    mint_token(deps.storage, msg.token_id.to_string(), token)?;
    increment_num_tokens(deps.storage)?;

    let modules = read_modules(deps.storage)?;
    let mod_res = modules
        .hook(|module| module.on_mint(&deps, info.clone(), env.clone(), msg.token_id.clone()))?;

    Ok(Response::default()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "mint"),
            attr("token_id", msg.token_id),
            attr("owner", info.sender.to_string()),
            attr("name", msg.name),
            attr("symbol", config.symbol),
            attr(
                "pricing",
                match msg.pricing {
                    Some(price) => price.to_string(),
                    None => String::from("none"),
                },
            ),
            attr(
                "metadata_type",
                match msg.metadata {
                    Some(metadata) => metadata.data_type.to_string(),
                    None => String::from("unspecified"),
                },
            ),
            attr("publisher", info.sender.to_string()),
            attr("description", msg.description.unwrap_or_default()),
            attr("token_uri", msg.token_uri.unwrap_or_default()),
        ]))
}

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let modules = read_modules(deps.storage)?;
    let mod_res = modules.hook(|module| {
        module.on_transfer(
            &deps,
            info.clone(),
            env.clone(),
            recipient.clone(),
            token_id.clone(),
        )
    })?;

    let res = transfer_nft(deps, &env, &info, &recipient, &token_id)?;

    Ok(res
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "transfer"),
            attr("recipient", recipient),
            attr("token_id", token_id),
            attr("sender", info.sender.to_string()),
        ]))
}

pub fn execute_send_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    token_id: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    let modules = read_modules(deps.storage)?;
    let mod_res = modules.hook(|module| {
        module.on_send(
            &deps,
            info.clone(),
            env.clone(),
            contract.clone(),
            token_id.clone(),
        )
    })?;

    // Transfer token
    let res = transfer_nft(deps, &env, &info, &contract, &token_id)?;

    let send = Cw721ReceiveMsg {
        sender: info.sender.to_string(),
        token_id: token_id.to_string(),
        msg,
    };

    // Send message
    Ok(res
        .add_message(send.into_cosmos_msg(contract.clone())?)
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "send"),
            attr("sender", info.sender),
            attr("recipient", contract),
            attr("token_id", token_id),
        ]))
}

pub fn execute_approve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    spender: String,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    let modules = read_modules(deps.storage)?;
    let mod_res = modules.hook(|module| {
        module.on_approve(
            &deps,
            info.clone(),
            env.clone(),
            spender.clone(),
            token_id.clone(),
            expires,
        )
    })?;

    let spender_addr = deps.api.addr_validate(&spender)?;
    let approval = Approval {
        spender: spender_addr,
        expires: expires.unwrap_or_default(),
    };

    add_approval(deps, &info, token_id.clone(), approval)?;

    Ok(Response::default()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "approve"),
            attr("token_id", token_id),
            attr("spender", spender),
        ]))
}

pub fn execute_revoke(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    spender: String,
) -> Result<Response, ContractError> {
    let modules = read_modules(deps.storage)?;
    let mod_res = modules.hook(|module| {
        module.on_revoke(
            &deps,
            info.clone(),
            env.clone(),
            spender.clone(),
            token_id.clone(),
        )
    })?;

    let spender_addr = deps.api.addr_validate(&spender)?;

    remove_approval(deps, &info, token_id.clone(), &spender_addr)?;

    Ok(Response::default()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "revoke"),
            attr("token_id", token_id),
            attr("spender", spender),
        ]))
}

fn execute_approve_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operator: String,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    let modules = read_modules(deps.storage)?;
    let mod_res = modules.hook(|module| {
        module.on_approve_all(&deps, info.clone(), env.clone(), operator.clone(), expires)
    })?;

    OPERATOR.save(
        deps.storage,
        (info.sender.to_string(), operator.clone()),
        &expires.unwrap_or_default(),
    )?;

    Ok(Response::default()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "approve_all"),
            attr("operator", operator),
            attr("sender", info.sender.to_string()),
        ]))
}

fn execute_revoke_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operator: String,
) -> Result<Response, ContractError> {
    let modules = read_modules(deps.storage)?;
    let mod_res = modules
        .hook(|module| module.on_revoke_all(&deps, info.clone(), env.clone(), operator.clone()))?;

    OPERATOR.remove(deps.storage, (info.sender.to_string(), operator.clone()));

    Ok(Response::default()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "revoke_all"),
            attr("operator", operator),
            attr("sender", info.sender.to_string()),
        ]))
}

fn execute_transfer_agreement(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    purchaser: String,
    amount: u128,
    denom: String,
) -> Result<Response, ContractError> {
    let modules = read_modules(deps.storage)?;
    let mod_res = modules.hook(|module| {
        module.on_transfer_agreement(
            &deps,
            info.clone(),
            env.clone(),
            token_id.clone(),
            purchaser.clone(),
            coin(amount, denom.clone()),
        )
    })?;
    let mut token = load_token(deps.storage, token_id.clone())?;
    let mut condition = info.sender.to_string().eq(&token.owner);
    // If auction is a module for this token we whitelist it to be able to make transfer agreements.
    let auction_contract = AUCTION_CONTRACT.may_load(deps.storage)?;
    if let Some(auction_contract) = auction_contract {
        condition = condition || info.sender.to_string().eq(&auction_contract);
    }
    require(condition, ContractError::Unauthorized {})?;
    require(!token.archived, ContractError::TokenIsArchived {})?;

    let amount = coin(amount, denom);
    let agreement = TransferAgreement {
        purchaser: purchaser.clone(),
        amount: amount.clone(),
    };
    token.transfer_agreement = Some(agreement);

    tokens().save(deps.storage, token_id.clone(), &Some(token))?;

    Ok(Response::default()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "transfer_agreement"),
            attr("purchaser", purchaser),
            attr("amount", amount.to_string()),
            attr("token_id", token_id),
        ]))
}

fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let token = load_token(deps.storage, token_id.clone())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        ContractError::Unauthorized {},
    )?;
    require(!token.archived, ContractError::TokenIsArchived {})?;

    let modules = read_modules(deps.storage)?;
    // let mod_res = modules.on_burn(&deps, info.clone(), env.clone(), token_id.clone())?;
    let mod_res = modules
        .hook(|module| module.on_burn(&deps, info.clone(), env.clone(), token_id.clone()))?;

    tokens().remove(deps.storage, token_id.clone())?;
    decrement_num_tokens(deps.storage)?;

    Ok(Response::default()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "burn"),
            attr("token_id", token_id),
            attr("sender", info.sender.to_string()),
        ]))
}

fn execute_archive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let mut token = load_token(deps.storage, token_id.clone())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        ContractError::Unauthorized {},
    )?;
    require(!token.archived, ContractError::TokenIsArchived {})?;
    let modules = read_modules(deps.storage)?;
    let mod_res = modules
        .hook(|module| module.on_archive(&deps, info.clone(), env.clone(), token_id.clone()))?;

    token.archived = true;
    tokens().save(deps.storage, token_id.clone(), &Some(token))?;

    Ok(Response::default()
        .add_submessages(mod_res.msgs)
        .add_events(mod_res.events)
        .add_attributes(vec![
            attr("action", "archive"),
            attr("token_id", token_id),
            attr("sender", info.sender.to_string()),
        ]))
}

fn execute_update_pricing(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    pricing: Option<Coin>,
) -> Result<Response, ContractError> {
    let mut token = load_token(deps.storage, token_id.clone())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        ContractError::Unauthorized {},
    )?;
    require(!token.archived, ContractError::TokenIsArchived {})?;

    token.pricing = pricing.clone();
    tokens().save(deps.storage, token_id.clone(), &Some(token))?;

    Ok(Response::default().add_attributes(vec![
        attr("action", "update_pricing"),
        attr("token_id", token_id),
        attr(
            "pricing",
            match pricing {
                Some(price) => price.to_string(),
                None => String::from("none"),
            },
        ),
    ]))
}

fn transfer_nft(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    recipient: &str,
    token_id: &str,
) -> Result<Response, ContractError> {
    let mut token = load_token(deps.storage, token_id.to_string())?;
    require(
        has_transfer_rights(deps.storage, env, info.sender.to_string(), &token)?,
        ContractError::Unauthorized {},
    )?;
    require(!token.archived, ContractError::TokenIsArchived {})?;
    let owner = token.owner;

    //Validate recipient is valid address
    deps.api.addr_validate(recipient)?;
    token.owner = recipient.to_string();
    token.approvals = vec![];

    let mut res = Response::new();

    // [GLOBAL-02] Changing is_some() + .unwrap() to if let Some()
    if let Some(transfer_agreement) = token.transfer_agreement.clone() {
        res = transfer_agreement.on_transfer(&deps, info, env, owner, res)?;
    }
    token.transfer_agreement = None;

    tokens().save(deps.storage, token_id.to_string(), &Some(token))?;
    Ok(res)
}

fn add_approval(
    deps: DepsMut,
    info: &MessageInfo,
    token_id: String,
    approval: Approval,
) -> Result<(), ContractError> {
    let mut token = load_token(deps.storage, token_id.to_string())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        ContractError::Unauthorized {},
    )?;
    require(!token.archived, ContractError::TokenIsArchived {})?;

    token.filter_approval(&approval.spender);

    token.approvals.push(approval);
    tokens().save(deps.storage, token_id, &Some(token))?;
    Ok(())
}

fn remove_approval(
    deps: DepsMut,
    info: &MessageInfo,
    token_id: String,
    spender: &Addr,
) -> Result<(), ContractError> {
    let mut token = load_token(deps.storage, token_id.to_string())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        ContractError::Unauthorized {},
    )?;
    require(!token.archived, ContractError::TokenIsArchived {})?;

    token.filter_approval(spender);

    tokens().save(deps.storage, token_id, &Some(token))?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => handle_andromeda_query(deps, env, msg),
        QueryMsg::OwnerOf { token_id } => encode_binary(&query_owner(deps, env, token_id)?),
        QueryMsg::ApprovedForAll {
            start_after,
            owner,
            include_expired,
            limit,
        } => encode_binary(&query_all_approvals(
            deps,
            env,
            owner,
            include_expired.unwrap_or_default(),
            start_after,
            limit,
        )?),
        QueryMsg::NumTokens {} => encode_binary(&query_num_tokens(deps, env)?),
        QueryMsg::NftInfo { token_id } => encode_binary(&query_nft_info(deps, token_id)?),
        QueryMsg::AllNftInfo { token_id } => {
            encode_binary(&query_all_nft_info(deps, env, token_id)?)
        }
        QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => encode_binary(&query_owned_tokens(deps, owner, start_after, limit)?),
        QueryMsg::AllTokens { start_after, limit } => {
            encode_binary(&query_all_tokens(deps, start_after, limit)?)
        }
        QueryMsg::ContractInfo {} => encode_binary(&query_contract_info(deps)?),
        QueryMsg::ModuleInfo {} => encode_binary(&query_module_info(deps)?),
    }
}

fn handle_andromeda_query(
    deps: Deps,
    env: Env,
    msg: AndromedaQuery,
) -> Result<Binary, ContractError> {
    match msg {
        AndromedaQuery::Get(data) => {
            let received = parse_message(data)?;
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

fn query_owner(deps: Deps, _env: Env, token_id: String) -> Result<OwnerOfResponse, ContractError> {
    let token = load_token(deps.storage, token_id)?;
    Ok(OwnerOfResponse {
        owner: token.owner.clone(),
        approvals: humanize_approvals(&token),
    })
}

fn query_all_approvals(
    deps: Deps,
    env: Env,
    owner: String,
    include_expired: bool,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<ApprovedForAllResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_addr.map(|addr| Bound::exclusive(addr.as_ref()));

    let res: StdResult<Vec<_>> = OPERATOR
        .prefix(owner)
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|r| include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block))
        .take(limit)
        .map(parse_approval)
        .collect();
    Ok(ApprovedForAllResponse { operators: res? })
}

fn query_num_tokens(deps: Deps, _env: Env) -> Result<NumTokensResponse, ContractError> {
    let num_tokens = NUM_TOKENS.load(deps.storage).unwrap_or_default();
    Ok(NumTokensResponse { count: num_tokens })
}

fn query_nft_info(
    deps: Deps,
    token_id: String,
) -> Result<NftInfoResponse<NftInfoResponseExtension>, ContractError> {
    let token = load_token(deps.storage, token_id)?;
    let extension = NftInfoResponseExtension {
        metadata: token.metadata,
        archived: token.archived,
        transfer_agreement: token.transfer_agreement,
        pricing: token.pricing,
    };

    Ok(NftInfoResponse {
        // name: token.name,
        // description: token.description.unwrap_or_default(),
        token_uri: token.token_uri,
        extension,
    })
}

fn query_all_nft_info(
    deps: Deps,
    env: Env,
    token_id: String,
) -> Result<AllNftInfoResponse<NftInfoResponseExtension>, ContractError> {
    let access = query_owner(deps, env, token_id.clone())?;
    let info = query_nft_info(deps, token_id)?;

    Ok(AllNftInfoResponse { access, info })
}

fn query_contract_info(deps: Deps) -> Result<ContractInfoResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ContractInfoResponse {
        name: config.name,
        symbol: config.symbol,
    })
}

fn query_module_info(deps: Deps) -> Result<ModuleInfoResponse, ContractError> {
    let modules = read_modules(deps.storage)?;
    let contracts: Vec<ModuleContract> = modules
        .module_defs
        .iter()
        .map(|def| {
            let name = def.name();
            let addr = def.as_module().get_contract_address(deps.storage);

            ModuleContract {
                module: name,
                contract: addr,
            }
        })
        .collect();

    Ok(ModuleInfoResponse {
        modules: modules.module_defs,
        contracts,
    })
}

//Queries partially taken from CW721-base
//https://github.com/CosmWasm/cw-nfts/blob/main/contracts/cw721-base/src/query.rs
fn query_owned_tokens(
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<TokensResponse, ContractError> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let pks: Vec<_> = tokens()
        .idx
        .owner
        .prefix(owner_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect();

    let res: Result<Vec<_>, _> = pks.iter().map(|v| String::from_utf8(v.to_vec())).collect();
    let tokens = res.map_err(StdError::invalid_utf8)?;

    Ok(TokensResponse { tokens })
}

fn query_all_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<TokensResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let all_tokens: StdResult<Vec<String>> = tokens()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(k, _)| String::from_utf8_lossy(&k).to_string()))
        .collect();

    Ok(TokensResponse {
        tokens: all_tokens?,
    })
}

/**
The next few functions are taken from the CW721-base contract:
https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw721-base
*/
fn parse_approval(item: StdResult<Pair<Expiration>>) -> StdResult<cw721::Approval> {
    item.and_then(|(k, expires)| {
        let spender = String::from_utf8(k)?;
        Ok(cw721::Approval { spender, expires })
    })
}

pub fn maybe_addr(api: &dyn Api, human: Option<String>) -> StdResult<Option<Addr>> {
    human.map(|x| api.addr_validate(&x)).transpose()
}

fn humanize_approvals(info: &Token) -> Vec<cw721::Approval> {
    info.approvals.iter().map(humanize_approval).collect()
}

fn humanize_approval(approval: &Approval) -> cw721::Approval {
    cw721::Approval {
        spender: approval.spender.to_string(),
        expires: approval.expires,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::{
        modules::auction::AUCTION_CONTRACT,
        testing::mock_querier::{mock_dependencies_custom, MOCK_AUCTION_CONTRACT},
        token::{Approval, ExecuteMsg},
    };
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Api, BankMsg, Uint128,
    };

    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "T";
    const BLACK_LIST_ADDRESS: &str = "blacklisted";

    fn store_mock_config(deps: DepsMut, minter: String) {
        CONFIG
            .save(
                deps.storage,
                &TokenConfig {
                    name: TOKEN_NAME.to_string(),
                    symbol: TOKEN_SYMBOL.to_string(),
                    minter,
                },
            )
            .unwrap()
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            modules: vec![],
            minter: String::from("creator"),
        };

        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
    //Added a test to check blacklist
    #[test]
    fn test_instantiate_blacklist() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            modules: vec![],
            minter: String::from(BLACK_LIST_ADDRESS),
        };

        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
        let err = ContractError::MinterBlacklisted {};
        assert_eq!(err, res);
    }

    #[test]
    fn test_mint() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("minter", &[]);
        let token_id = String::default();
        let creator = "creator".to_string();

        let mint_msg = MintMsg {
            token_id: token_id.clone(),
            owner: creator.clone(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        };

        store_mock_config(deps.as_mut(), String::from("minter"));

        let msg = ExecuteMsg::Mint(Box::new(mint_msg));

        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let query_msg = QueryMsg::OwnerOf { token_id };

        let query_res = query(deps.as_ref(), env, query_msg).unwrap();
        let query_val: OwnerOfResponse = from_binary(&query_res).unwrap();

        assert_eq!(query_val.owner, creator)
    }

    #[test]
    fn test_mint_invalid_minter() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let token_id = String::default();
        let creator = "creator".to_string();

        let mint_msg = MintMsg {
            token_id,
            owner: creator,
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        };

        store_mock_config(deps.as_mut(), String::from("minter"));

        let msg = ExecuteMsg::Mint(Box::new(mint_msg));

        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_transfer() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let recipient = "recipient";
        let info = mock_info(minter, &[]);
        let token_id = String::default();
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: token_id.clone(),
        };
        let attrs = vec![
            attr("action", "transfer"),
            attr("recipient", recipient),
            attr("token_id", token_id.clone()),
            attr("sender", info.sender.to_string()),
        ];

        //store config
        store_mock_config(deps.as_mut(), minter.to_string());

        let token = Token {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: String::default(),
            approvals: vec![],
            transfer_agreement: None,
            metadata: None,
            token_uri: None,
            archived: false,
            pricing: None,
            publisher: minter.to_string(),
        };

        tokens()
            .save(deps.as_mut().storage, token_id.to_string(), &Some(token))
            .unwrap();

        let unauth_info = mock_info("anyone", &[]);

        let unauth_res = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
        assert_eq!(unauth_res, ContractError::Unauthorized {});

        let notfound_msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: String::from("2"),
        };
        let notfound_res =
            execute(deps.as_mut(), env.clone(), info.clone(), notfound_msg).unwrap_err();

        assert_eq!(
            notfound_res,
            ContractError::Std(StdError::not_found(
                "core::option::Option<andromeda_protocol::token::Token>"
            ))
        );

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(Response::default().add_attributes(attrs), res);

        let owner = load_token(deps.as_ref().storage, token_id).unwrap().owner;
        assert_eq!(recipient.to_string(), owner);
    }

    #[test]
    fn test_transfer_approval() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let recipient = "recipient";

        //store config
        store_mock_config(deps.as_mut(), minter.to_string());

        let approval_info = mock_info("spender", &[]);
        let approval = Approval {
            spender: approval_info.sender.clone(),
            expires: cw721::Expiration::Never {},
        };
        let approval_token_id = String::from("2");
        let approval_token = Token {
            token_id: approval_token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: String::default(),
            approvals: vec![approval],
            transfer_agreement: None,
            metadata: None,
            archived: false,
            token_uri: None,
            pricing: None,
            publisher: minter.to_string(),
        };
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: approval_token_id.clone(),
        };

        tokens()
            .save(
                deps.as_mut().storage,
                approval_token_id.to_string(),
                &Some(approval_token),
            )
            .unwrap();

        let res = execute(deps.as_mut(), env, approval_info.clone(), msg).unwrap();
        assert_eq!(
            Response::default().add_attributes(vec![
                attr("action", "transfer"),
                attr("recipient", recipient),
                attr("token_id", approval_token_id.clone()),
                attr("sender", approval_info.sender.to_string()),
            ]),
            res
        );
        let owner = load_token(deps.as_ref().storage, approval_token_id)
            .unwrap()
            .owner;
        assert_eq!(recipient.to_string(), owner);
    }

    #[test]
    fn test_agreed_transfer() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let recipient = "recipient";
        let info = mock_info(minter, &[]);
        let token_id = String::default();
        //store config
        store_mock_config(deps.as_mut(), minter.to_string());

        let msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: token_id.clone(),
        };
        let amount = coin(100, "uluna");

        let token = Token {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: String::default(),
            approvals: vec![],
            transfer_agreement: Some(TransferAgreement {
                purchaser: recipient.to_string(),
                amount: amount.clone(),
            }),
            metadata: None,
            archived: false,
            token_uri: None,
            pricing: None,
            publisher: minter.to_string(),
        };

        tokens()
            .save(
                deps.as_mut().storage,
                token_id.to_string(),
                &Some(token.clone()),
            )
            .unwrap();

        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(
            Response::new()
                .add_message(BankMsg::Send {
                    to_address: minter.to_string(),
                    amount: vec![amount]
                })
                .add_event(token.transfer_agreement.unwrap().generate_event())
                .add_attributes(vec![
                    attr("action", "transfer"),
                    attr("recipient", recipient),
                    attr("token_id", token_id),
                    attr("sender", info.sender.to_string()),
                ]),
            res
        );
    }

    #[test]
    fn test_approve() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let sender = "sender";
        let info = mock_info(sender, &[]);
        let token_id = String::default();
        let approvee = "aprovee";

        let msg = ExecuteMsg::Approve {
            spender: approvee.to_string(),
            expires: None,
            token_id: String::default(),
        };

        let token = Token {
            token_id: token_id.clone(),
            description: None,
            name: String::default(),
            approvals: vec![],
            owner: sender.to_string(),
            transfer_agreement: None,
            metadata: None,
            archived: false,
            token_uri: None,
            pricing: None,
            publisher: sender.to_string(),
        };

        tokens()
            .save(deps.as_mut().storage, token_id.to_string(), &Some(token))
            .unwrap();

        execute(deps.as_mut(), env, info, msg).unwrap();
        let token = load_token(deps.as_mut().storage, token_id).unwrap();

        assert_eq!(1, token.approvals.len());
        assert_eq!(approvee, token.approvals[0].spender.to_string());
    }

    #[test]
    fn test_revoke() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let sender = "sender";
        let info = mock_info(sender, &[]);
        let token_id = String::default();
        let approvee = "aprovee";
        let approval = Approval {
            expires: Expiration::Never {},
            spender: deps.api.addr_validate(approvee).unwrap(),
        };

        let msg = ExecuteMsg::Revoke {
            spender: approvee.to_string(),
            token_id: String::default(),
        };

        let token = Token {
            token_id: token_id.clone(),
            description: None,
            name: String::default(),
            approvals: vec![approval],
            owner: sender.to_string(),
            transfer_agreement: None,
            metadata: None,
            archived: false,
            token_uri: None,
            pricing: None,
            publisher: sender.to_string(),
        };

        tokens()
            .save(deps.as_mut().storage, token_id.to_string(), &Some(token))
            .unwrap();

        execute(deps.as_mut(), env, info, msg).unwrap();
        let token = load_token(deps.as_mut().storage, token_id).unwrap();

        assert_eq!(0, token.approvals.len());
    }

    #[test]
    fn test_approve_all() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter, &[]);
        let token_id = String::default();
        let operator = "operator";
        let operator_info = mock_info(operator, &[]);
        //store config
        store_mock_config(deps.as_mut(), minter.to_string());

        let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        }));
        execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: operator.to_string(),
            token_id: token_id.clone(),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            operator_info.clone(),
            transfer_msg,
        )
        .unwrap_err();

        assert_eq!(err, ContractError::Unauthorized {});

        let approve_all_msg = ExecuteMsg::ApproveAll {
            operator: operator.to_string(),
            expires: None,
        };
        execute(deps.as_mut(), env.clone(), info, approve_all_msg).unwrap();

        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: operator.to_string(),
            token_id: token_id.clone(),
        };
        execute(deps.as_mut(), env, operator_info, transfer_msg).unwrap();

        let token = load_token(deps.as_ref().storage, token_id).unwrap();

        assert_eq!(token.owner, operator.to_string());
    }

    #[test]
    fn test_revoke_all() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter, &[]);
        let token_id = String::default();
        let operator = "operator";
        let operator_info = mock_info(operator, &[]);

        //store config
        store_mock_config(deps.as_mut(), minter.to_string());

        let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        }));
        execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

        let approve_all_msg = ExecuteMsg::ApproveAll {
            operator: operator.to_string(),
            expires: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), approve_all_msg).unwrap();

        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: minter.to_string(),
            token_id: token_id.clone(),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            operator_info.clone(),
            transfer_msg,
        )
        .unwrap();

        let revoke_msg = ExecuteMsg::RevokeAll {
            operator: operator.to_string(),
        };
        execute(deps.as_mut(), env.clone(), info, revoke_msg).unwrap();

        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: minter.to_string(),
            token_id,
        };
        let err = execute(deps.as_mut(), env, operator_info, transfer_msg).unwrap_err();

        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn test_transfer_agreement() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let purchaser = "purchaser";
        let info = mock_info(minter, &[]);
        let token_id = String::default();
        let denom = "uluna";
        let amount = Uint128::from(100u64);
        let metadata = None;

        let instantiate_msg = InstantiateMsg {
            name: "Token Name".to_string(),
            symbol: "TS".to_string(),
            minter: minter.to_string(),
            modules: vec![],
        };
        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
            metadata,
            token_uri: None,
            pricing: None,
        }));
        execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

        let transfer_agreement_msg = ExecuteMsg::TransferAgreement {
            token_id: token_id.clone(),
            denom: denom.to_string(),
            amount,
            purchaser: purchaser.to_string(),
        };
        execute(deps.as_mut(), env.clone(), info, transfer_agreement_msg).unwrap();

        let agreement_query = QueryMsg::NftInfo { token_id };
        let res = query(deps.as_ref(), env, agreement_query).unwrap();
        let token_res: NftInfoResponse<NftInfoResponseExtension> = from_binary(&res).unwrap();
        let agreement = token_res.extension.transfer_agreement.unwrap();

        assert_eq!(agreement.purchaser, purchaser);
        assert_eq!(agreement.amount, coin(amount.u128(), denom))
    }

    #[test]
    fn test_transfer_agreement_as_auction_contract() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let minter = "minter";
        let purchaser = "purchaser";
        let info = mock_info(minter, &[]);
        let token_id = String::default();
        let denom = "uluna";
        let amount = Uint128::from(100u64);
        let metadata = None;

        let instantiate_msg = InstantiateMsg {
            name: "Token Name".to_string(),
            symbol: "TS".to_string(),
            minter: minter.to_string(),
            modules: vec![],
        };
        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
            metadata,
            token_uri: None,
            pricing: None,
        }));
        execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

        AUCTION_CONTRACT
            .save(deps.as_mut().storage, &MOCK_AUCTION_CONTRACT.to_string())
            .unwrap();
        let transfer_agreement_msg = ExecuteMsg::TransferAgreement {
            token_id: token_id.clone(),
            denom: denom.to_string(),
            amount,
            purchaser: purchaser.to_string(),
        };
        let info = mock_info(MOCK_AUCTION_CONTRACT, &[]);
        execute(deps.as_mut(), env.clone(), info, transfer_agreement_msg).unwrap();

        let agreement_query = QueryMsg::NftInfo { token_id };
        let res = query(deps.as_ref(), env, agreement_query).unwrap();
        let token_res: NftInfoResponse<NftInfoResponseExtension> = from_binary(&res).unwrap();
        let agreement = token_res.extension.transfer_agreement.unwrap();

        assert_eq!(agreement.purchaser, purchaser);
        assert_eq!(agreement.amount, coin(amount.u128(), denom))
    }

    #[test]
    fn test_execute_burn() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter, &[]);
        let token_id = "1";
        store_mock_config(deps.as_mut(), minter.to_string());

        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        };

        let msg = ExecuteMsg::Mint(Box::new(mint_msg));

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let unauth_info = mock_info("anyone", &[]);
        let burn_msg = ExecuteMsg::Burn {
            token_id: token_id.to_string(),
        };

        let resp = execute(deps.as_mut(), env.clone(), unauth_info, burn_msg.clone()).unwrap_err();

        assert_eq!(resp, ContractError::Unauthorized {});

        execute(deps.as_mut(), env.clone(), info, burn_msg).unwrap();

        let query_msg = QueryMsg::OwnerOf {
            token_id: token_id.to_string(),
        };

        let query_res = query(deps.as_ref(), env, query_msg).unwrap_err();

        assert_eq!(
            query_res,
            ContractError::Std(StdError::not_found(
                "core::option::Option<andromeda_protocol::token::Token>"
            ))
        )
    }

    #[test]
    fn test_execute_archive() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter, &[]);
        let token_id = "1";
        store_mock_config(deps.as_mut(), minter.to_string());

        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        };

        let msg = ExecuteMsg::Mint(Box::new(mint_msg));

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let unauth_info = mock_info("anyone", &[]);
        let archive_msg = ExecuteMsg::Archive {
            token_id: token_id.to_string(),
        };

        let resp =
            execute(deps.as_mut(), env.clone(), unauth_info, archive_msg.clone()).unwrap_err();

        assert_eq!(resp, ContractError::Unauthorized {});

        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            archive_msg.clone(),
        )
        .unwrap();

        let archived_resp = execute(deps.as_mut(), env.clone(), info, archive_msg).unwrap_err();
        assert_eq!(archived_resp, ContractError::TokenIsArchived {});

        let token_query = QueryMsg::NftInfo {
            token_id: token_id.to_string(),
        };
        let res = query(deps.as_ref(), env, token_query).unwrap();
        let token_res: NftInfoResponse<NftInfoResponseExtension> = from_binary(&res).unwrap();
        assert!(token_res.extension.archived)
    }

    #[test]
    fn test_execute_update_pricing() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter, &[]);
        let token_id = "1";
        store_mock_config(deps.as_mut(), minter.to_string());

        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        };

        let msg = ExecuteMsg::Mint(Box::new(mint_msg));

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let unauth_info = mock_info("anyone", &[]);
        let pricing = coin(100, "uluna");
        let update_msg = ExecuteMsg::UpdatePricing {
            token_id: token_id.to_string(),
            price: Some(pricing.clone()),
        };

        let resp =
            execute(deps.as_mut(), env.clone(), unauth_info, update_msg.clone()).unwrap_err();
        assert_eq!(resp, ContractError::Unauthorized {});

        let resp = execute(deps.as_mut(), env.clone(), info, update_msg).unwrap();
        let expected = Response::default().add_attributes(vec![
            attr("action", "update_pricing"),
            attr("token_id", token_id),
            attr("pricing", pricing.to_string()),
        ]);
        assert_eq!(expected, resp);

        let token_query = QueryMsg::NftInfo {
            token_id: token_id.to_string(),
        };
        let res = query(deps.as_ref(), env, token_query).unwrap();
        let token_res: NftInfoResponse<NftInfoResponseExtension> = from_binary(&res).unwrap();
        assert_eq!(token_res.extension.pricing.unwrap(), pricing)
    }

    #[test]
    fn test_owned_tokens() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter, &[]);
        let token_id = "1";
        store_mock_config(deps.as_mut(), minter.to_string());

        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        };

        let msg = ExecuteMsg::Mint(Box::new(mint_msg));

        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let query_msg = QueryMsg::Tokens {
            owner: minter.to_string(),
            limit: Some(1),
            start_after: None,
        };
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: TokensResponse = from_binary(&res).unwrap();

        assert_eq!(val.tokens, vec![token_id.to_string()])
    }

    #[test]
    fn test_all_tokens() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter, &[]);
        let token_id = "1";
        store_mock_config(deps.as_mut(), minter.to_string());

        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        };

        let msg = ExecuteMsg::Mint(Box::new(mint_msg));

        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let query_msg = QueryMsg::AllTokens {
            limit: Some(1),
            start_after: None,
        };
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let val: TokensResponse = from_binary(&res).unwrap();

        assert_eq!(val.tokens, vec![token_id.to_string()])
    }

    #[test]
    fn test_andr_receive() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let token_id = String::default();
        let creator = "creator".to_string();

        let mint_msg = MintMsg {
            token_id: token_id.clone(),
            owner: creator.clone(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
            token_uri: None,
            pricing: None,
        };

        store_mock_config(deps.as_mut(), String::from("creator"));

        let msg = ExecuteMsg::Mint(Box::new(mint_msg));

        let msg =
            ExecuteMsg::AndrReceive(AndromedaMsg::Receive(Some(encode_binary(&msg).unwrap())));

        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let query_msg = QueryMsg::OwnerOf { token_id };
        let query_msg = QueryMsg::AndrQuery(AndromedaQuery::Get(Some(
            encode_binary(&query_msg).unwrap(),
        )));

        let query_res = query(deps.as_ref(), env, query_msg).unwrap();
        let query_val: OwnerOfResponse = from_binary(&query_res).unwrap();

        assert_eq!(query_val.owner, creator)
    }
}
