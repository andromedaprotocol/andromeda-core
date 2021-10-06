use andromeda_protocol::{
    modules::{
        address_list::{on_address_list_reply, REPLY_ADDRESS_LIST},
        common::require,
        read_modules,
        receipt::{get_receipt_module, on_receipt_reply, REPLY_RECEIPT},
        store_modules, Modules,
    },
    token::{
        Approval, ExecuteMsg, InstantiateMsg, MigrateMsg, MintMsg, NftArchivedResponse,
        NftMetadataResponse, NftTransferAgreementResponse, QueryMsg, Token, TokenId,
        TransferAgreement,
    },
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, Order,
    Pair, Reply, Response, StdError, StdResult,
};
use cw721::{
    AllNftInfoResponse, ApprovedForAllResponse, ContractInfoResponse, Cw721ReceiveMsg, Expiration,
    NftInfoResponse, NumTokensResponse, OwnerOfResponse,
};
use cw_storage_plus::Bound;

use crate::state::{
    decrement_num_tokens, has_transfer_rights, increment_num_tokens, TokenConfig, CONFIG,
    NUM_TOKENS, OPERATOR, TOKENS,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    msg.validate()?;

    let config = TokenConfig {
        name: msg.name,
        symbol: msg.symbol,
        minter: msg.minter.to_string(),
        metadata_limit: msg.metadata_limit,
    };

    CONFIG.save(deps.storage, &config)?;
    let modules = Modules::new(msg.modules);
    store_modules(deps.storage, modules.clone())?;

    let mut resp = Response::new();

    let mod_res = modules.on_instantiate(&deps, info.clone(), env)?;
    resp = resp.add_submessages(mod_res.msgs);

    if msg.init_hook.is_some() {
        let hook = msg.init_hook.unwrap();
        resp = resp.add_message(hook.into_cosmos_msg(info.sender.to_string())?);
    }

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        REPLY_RECEIPT => on_receipt_reply(deps, msg),
        REPLY_ADDRESS_LIST => on_address_list_reply(deps, msg),
        _ => Err(StdError::generic_err("reply id is invalid")),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    modules.on_execute(&deps, info.clone(), env.clone())?;

    match msg {
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
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MintMsg,
) -> StdResult<Response> {
    let config = CONFIG.may_load(deps.storage)?;

    let metadata = match msg.metadata {
        Some(data) => {
            if config.is_some() {
                config.unwrap().validate_metadata_size(data.clone())?;
            }
            Some(to_binary(&data)?)
        }
        None => None,
    };

    let token = Token {
        token_id: msg.token_id.clone(),
        owner: info.sender.to_string(),
        description: msg.description,
        name: msg.name,
        approvals: vec![],
        transfer_agreement: None,
        metadata,
        archived: false,
    };

    TOKENS.save(deps.storage, msg.token_id.to_string(), &token)?;
    increment_num_tokens(deps.storage)?;

    let modules = read_modules(deps.storage)?;
    modules.on_mint(&deps, info.clone(), env.clone(), msg.token_id.clone())?;

    Ok(Response::default())
}

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    modules.on_transfer(
        &deps,
        info.clone(),
        env.clone(),
        recipient.clone(),
        token_id.clone(),
    )?;

    let res = transfer_nft(deps, &env, &info, &recipient, &token_id)?;

    Ok(res)
}

pub fn execute_send_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    token_id: String,
    msg: Binary,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    modules.on_send(
        &deps,
        info.clone(),
        env.clone(),
        contract.clone(),
        token_id.clone(),
    )?;

    // Transfer token
    transfer_nft(deps, &env, &info, &contract, &token_id)?;

    let send = Cw721ReceiveMsg {
        sender: info.sender.to_string(),
        token_id: token_id.to_string(),
        msg,
    };

    // Send message
    Ok(Response::new()
        .add_message(send.into_cosmos_msg(contract.clone())?)
        .add_attribute("action", "send_nft")
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", contract)
        .add_attribute("token_id", token_id.to_string()))
}

pub fn execute_approve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    spender: String,
    expires: Option<Expiration>,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    modules.on_approve(
        &deps,
        info.clone(),
        env.clone(),
        spender.clone(),
        token_id.clone(),
        expires.clone(),
    )?;

    let spender_addr = deps.api.addr_validate(&spender)?;
    let approval = Approval {
        spender: spender_addr,
        expires: expires.unwrap_or_default(),
    };

    add_approval(deps, &info, token_id, approval)?;

    Ok(Response::default())
}

pub fn execute_revoke(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    spender: String,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    modules.on_revoke(
        &deps,
        info.clone(),
        env.clone(),
        spender.clone(),
        token_id.clone(),
    )?;

    let spender_addr = deps.api.addr_validate(&spender)?;

    remove_approval(deps, &info, token_id, &spender_addr)?;

    Ok(Response::default())
}

fn execute_approve_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operator: String,
    expires: Option<Expiration>,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    modules.on_approve_all(
        &deps,
        info.clone(),
        env.clone(),
        operator.clone(),
        expires.clone(),
    )?;

    OPERATOR.save(
        deps.storage,
        (info.sender.to_string(), operator.clone()),
        &expires.unwrap_or_default(),
    )?;

    Ok(Response::default())
}

fn execute_revoke_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operator: String,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    modules.on_revoke_all(&deps, info.clone(), env.clone(), operator.clone())?;

    OPERATOR.remove(deps.storage, (info.sender.to_string(), operator.clone()));

    Ok(Response::default())
}

fn execute_transfer_agreement(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    purchaser: String,
    amount: u128,
    denom: String,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    modules.on_transfer_agreement(
        &deps,
        info.clone(),
        env.clone(),
        token_id.clone(),
        purchaser.clone(),
        amount.clone(),
        denom.clone(),
    )?;
    let mut token = TOKENS.load(deps.storage, token_id.clone())?;

    require(
        info.sender.to_string().eq(&token.owner.clone()),
        StdError::generic_err("Only the token owner can create a transfer agreement"),
    )?;
    require(
        !token.archived,
        StdError::generic_err("This token is archived and cannot be changed in any way."),
    )?;

    let agreement = TransferAgreement {
        purchaser: purchaser.clone(),
        amount: coin(amount, denom),
    };
    token.transfer_agreement = Some(agreement);

    TOKENS.save(deps.storage, token_id.clone(), &token)?;

    Ok(Response::default())
}

fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> StdResult<Response> {
    let token = TOKENS.load(deps.storage, token_id.clone())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        StdError::generic_err("Cannot burn a token you do not own"),
    )?;
    require(
        !token.archived,
        StdError::generic_err("This token is archived and cannot be changed in any way."),
    )?;

    let modules = read_modules(deps.storage)?;
    modules.on_burn(&deps, info.clone(), env.clone(), token_id.clone())?;

    TOKENS.remove(deps.storage, token_id.clone());
    decrement_num_tokens(deps.storage)?;

    Ok(Response::default())
}

fn execute_archive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> StdResult<Response> {
    let mut token = TOKENS.load(deps.storage, token_id.clone())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        StdError::generic_err("Cannot archive a token you do not own"),
    )?;
    require(
        !token.archived,
        StdError::generic_err("This token is archived and cannot be changed in any way."),
    )?;

    let modules = read_modules(deps.storage)?;
    modules.on_archive(&deps, info.clone(), env.clone(), token_id.clone())?;

    token.archived = true;
    TOKENS.save(deps.storage, token_id.clone(), &token)?;
    decrement_num_tokens(deps.storage)?;

    Ok(Response::default())
}

fn transfer_nft(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    recipient: &String,
    token_id: &String,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    let mut token = TOKENS.load(deps.storage, token_id.to_string())?;
    require(
        has_transfer_rights(deps.storage, env, info.sender.to_string(), &token)?,
        StdError::generic_err("Address does not have transfer rights for this token"),
    )?;
    require(
        !token.archived,
        StdError::generic_err("This token is archived and cannot be changed in any way."),
    )?;
    let owner = token.owner;

    token.owner = recipient.to_string();
    token.approvals = vec![];

    let mut res = match token.transfer_agreement.clone() {
        Some(agreement) => {
            let mut res = Response::new();
            let payment_message = agreement.generate_payment(owner.clone());
            let mut payments = vec![payment_message];
            let mod_resp = modules.on_agreed_transfer(
                &deps,
                info.clone(),
                env.clone(),
                &mut payments,
                owner.clone(),
                agreement.purchaser.clone(),
                agreement.amount.clone(),
            )?;

            for payment in payments {
                res = res.add_message(payment);
            }

            for event in &mod_resp.events {
                res = res.add_event(event.clone())
            }
            res = res.add_event(agreement.generate_event());

            let recpt_opt = get_receipt_module(deps.storage)?;
            match recpt_opt {
                Some(recpt_mod) => {
                    let recpt_msg =
                        recpt_mod.generate_receipt_message(deps.storage, res.events.clone())?;
                    res = res.add_message(recpt_msg);
                }
                None => {}
            }

            res
        }
        None => Response::default(),
    };

    res = res.add_attributes(vec![
        attr("action", "transfer"),
        attr("owner", owner.clone()),
        attr("recipient", recipient.clone()),
    ]);

    TOKENS.save(deps.storage, token_id.to_string(), &token)?;
    Ok(res)
}

fn add_approval(
    deps: DepsMut,
    info: &MessageInfo,
    token_id: TokenId,
    approval: Approval,
) -> StdResult<()> {
    let mut token = TOKENS.load(deps.storage, token_id.to_string())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        StdError::generic_err("Only the token owner can add approvals"),
    )?;
    require(
        !token.archived,
        StdError::generic_err("This token is archived and cannot be changed in any way."),
    )?;

    token.filter_approval(&approval.spender.clone());

    token.approvals.push(approval);
    TOKENS.save(deps.storage, token_id.to_string(), &token)?;
    Ok(())
}

fn remove_approval(
    deps: DepsMut,
    info: &MessageInfo,
    token_id: TokenId,
    spender: &Addr,
) -> StdResult<()> {
    let mut token = TOKENS.load(deps.storage, token_id.to_string())?;
    require(
        token.owner.eq(&info.sender.to_string()),
        StdError::generic_err("Only the token owner can add approvals"),
    )?;
    require(
        !token.archived,
        StdError::generic_err("This token is archived and cannot be changed in any way."),
    )?;

    token.filter_approval(spender);

    TOKENS.save(deps.storage, token_id.to_string(), &token)?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::OwnerOf { token_id } => to_binary(&query_owner(deps, env, token_id)?),
        QueryMsg::ApprovedForAll {
            start_after,
            owner,
            include_expired,
            limit,
        } => to_binary(&query_all_approvals(
            deps,
            env,
            owner,
            include_expired.unwrap_or_default(),
            start_after,
            limit,
        )?),
        QueryMsg::NumTokens {} => to_binary(&query_num_tokens(deps, env)?),
        QueryMsg::NftInfo { token_id } => to_binary(&query_nft_info(deps, token_id)?),
        QueryMsg::AllNftInfo { token_id } => to_binary(&query_all_nft_info(deps, env, token_id)?),
        QueryMsg::ContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::NftTransferAgreementInfo { token_id } => {
            to_binary(&query_transfer_agreement(deps, token_id)?)
        }
        QueryMsg::NftMetadata { token_id } => to_binary(&query_token_metadata(deps, token_id)?),
        QueryMsg::NftArchiveStatus { token_id } => {
            to_binary(&query_token_archive_status(deps, token_id)?)
        }
    }
}

fn query_owner(deps: Deps, _env: Env, token_id: TokenId) -> StdResult<OwnerOfResponse> {
    let token = TOKENS.load(deps.storage, token_id.to_string())?;
    Ok(OwnerOfResponse {
        owner: token.clone().owner,
        approvals: humanize_approvals(&token.clone()),
    })
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

fn query_all_approvals(
    deps: Deps,
    env: Env,
    owner: String,
    include_expired: bool,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ApprovedForAllResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_addr.map(|addr| Bound::exclusive(addr.as_ref()));

    let res: StdResult<Vec<_>> = OPERATOR
        .prefix(owner.clone())
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|r| include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block))
        .take(limit)
        .map(parse_approval)
        .collect();
    Ok(ApprovedForAllResponse { operators: res? })
}

fn query_num_tokens(deps: Deps, _env: Env) -> StdResult<NumTokensResponse> {
    let num_tokens = NUM_TOKENS.load(deps.storage).unwrap_or_default();
    Ok(NumTokensResponse { count: num_tokens })
}

fn query_nft_info(deps: Deps, token_id: String) -> StdResult<NftInfoResponse> {
    let token = TOKENS.load(deps.storage, token_id.clone())?;

    Ok(NftInfoResponse {
        name: token.name,
        description: token.description.unwrap_or_default(),
        image: None,
    })
}

fn query_all_nft_info(deps: Deps, env: Env, token_id: String) -> StdResult<AllNftInfoResponse> {
    let access = query_owner(deps, env, token_id.clone())?;
    let info = query_nft_info(deps, token_id.clone())?;

    Ok(AllNftInfoResponse { access, info })
}

fn query_contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ContractInfoResponse {
        name: config.name,
        symbol: config.symbol,
    })
}

fn query_transfer_agreement(
    deps: Deps,
    token_id: String,
) -> StdResult<NftTransferAgreementResponse> {
    let token = TOKENS.load(deps.storage, token_id)?;

    Ok(NftTransferAgreementResponse {
        agreement: token.transfer_agreement,
    })
}

fn query_token_metadata(deps: Deps, token_id: String) -> StdResult<NftMetadataResponse> {
    let token = TOKENS.load(deps.storage, token_id)?;
    let metadata: Option<String> = match token.metadata {
        Some(data) => Some(from_binary(&data)?),
        None => None,
    };

    Ok(NftMetadataResponse { metadata })
}

fn query_token_archive_status(deps: Deps, token_id: String) -> StdResult<NftArchivedResponse> {
    let token = TOKENS.load(deps.storage, token_id)?;

    Ok(NftArchivedResponse {
        archived: token.archived,
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
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::token::Approval;
    use andromeda_protocol::token::ExecuteMsg;
    use andromeda_protocol::token::NftArchivedResponse;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Api, BankMsg, Uint128,
    };

    const ADDRESS_LIST_CODE_ID: u64 = 2;
    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "T";
    static RECEIPT_CODE_ID: u64 = 1;
    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            modules: vec![],
            receipt_code_id: RECEIPT_CODE_ID,
            minter: String::from("creator"),
            init_hook: None,
            metadata_limit: None,
            address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
        };

        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_mint() {
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
        };

        let msg = ExecuteMsg::Mint(mint_msg);

        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let query_msg = QueryMsg::OwnerOf { token_id };

        let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let query_val: OwnerOfResponse = from_binary(&query_res).unwrap();

        assert_eq!(query_val.owner, creator)
    }

    #[test]
    fn test_transfer() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let recipient = "recipient";
        let info = mock_info(minter.clone(), &[]);
        let token_id = String::default();
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: token_id.clone(),
        };
        let attrs = vec![
            attr("action", "transfer"),
            attr("owner", minter.to_string()),
            attr("recipient", recipient.to_string()),
        ];

        //store config
        CONFIG
            .save(
                deps.as_mut().storage,
                &TokenConfig {
                    name: TOKEN_NAME.to_string(),
                    symbol: TOKEN_SYMBOL.to_string(),
                    minter: String::from("creator"),
                    metadata_limit: None,
                },
            )
            .unwrap();

        let token = Token {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: String::default(),
            approvals: vec![],
            transfer_agreement: None,
            metadata: None,
            archived: false,
        };

        TOKENS
            .save(deps.as_mut().storage, token_id.to_string(), &token)
            .unwrap();

        let unauth_info = mock_info("anyone", &[]);

        let unauth_res =
            execute(deps.as_mut(), env.clone(), unauth_info.clone(), msg.clone()).unwrap_err();
        assert_eq!(
            unauth_res,
            StdError::generic_err("Address does not have transfer rights for this token")
        );

        let notfound_msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: String::from("2"),
        };
        let notfound_res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            notfound_msg.clone(),
        )
        .unwrap_err();

        assert_eq!(
            notfound_res,
            StdError::not_found("andromeda_protocol::token::Token")
        );

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(Response::default().add_attributes(attrs.clone()), res);
        let owner = TOKENS
            .load(deps.as_ref().storage, token_id.to_string())
            .unwrap()
            .owner;
        assert_eq!(recipient.to_string(), owner);

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
        };
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: approval_token_id.clone(),
        };

        TOKENS
            .save(
                deps.as_mut().storage,
                approval_token_id.to_string(),
                &approval_token,
            )
            .unwrap();

        let res = execute(
            deps.as_mut(),
            env.clone(),
            approval_info.clone(),
            msg.clone(),
        )
        .unwrap();
        assert_eq!(Response::default().add_attributes(attrs.clone()), res);
        let owner = TOKENS
            .load(deps.as_ref().storage, approval_token_id.to_string())
            .unwrap()
            .owner;
        assert_eq!(recipient.to_string(), owner);

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
        };
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: approval_token_id.clone(),
        };

        TOKENS
            .save(
                deps.as_mut().storage,
                approval_token_id.to_string(),
                &approval_token,
            )
            .unwrap();

        let res = execute(
            deps.as_mut(),
            env.clone(),
            approval_info.clone(),
            msg.clone(),
        )
        .unwrap();
        assert_eq!(Response::default().add_attributes(attrs.clone()), res);

        let owner = TOKENS
            .load(deps.as_ref().storage, approval_token_id.to_string())
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
        let info = mock_info(minter.clone(), &[]);
        let token_id = String::default();
        //store config
        CONFIG
            .save(
                deps.as_mut().storage,
                &TokenConfig {
                    name: TOKEN_NAME.to_string(),
                    symbol: TOKEN_SYMBOL.to_string(),
                    minter: String::from("creator"),
                    metadata_limit: None,
                },
            )
            .unwrap();

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
        };

        TOKENS
            .save(deps.as_mut().storage, token_id.to_string(), &token)
            .unwrap();

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(
            Response::new()
                .add_message(BankMsg::Send {
                    to_address: minter.to_string(),
                    amount: vec![amount.clone()]
                })
                .add_event(token.transfer_agreement.unwrap().generate_event())
                .add_attributes(vec![
                    attr("action", "transfer"),
                    attr("owner", minter.to_string()),
                    attr("recipient", recipient.to_string()),
                ]),
            res
        );
    }

    #[test]
    fn test_approve() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let sender = "sender";
        let info = mock_info(sender.clone(), &[]);
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
        };

        TOKENS
            .save(deps.as_mut().storage, token_id.to_string(), &token)
            .unwrap();

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let token = TOKENS
            .load(deps.as_mut().storage, token_id.to_string())
            .unwrap();

        assert_eq!(1, token.approvals.len());
        assert_eq!(approvee.clone(), token.approvals[0].spender.to_string());
    }

    #[test]
    fn test_revoke() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let sender = "sender";
        let info = mock_info(sender.clone(), &[]);
        let token_id = String::default();
        let approvee = "aprovee";
        let approval = Approval {
            expires: Expiration::Never {},
            spender: deps.api.addr_validate(approvee.clone()).unwrap(),
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
        };

        TOKENS
            .save(deps.as_mut().storage, token_id.to_string(), &token)
            .unwrap();

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let token = TOKENS
            .load(deps.as_mut().storage, token_id.to_string())
            .unwrap();

        assert_eq!(0, token.approvals.len());
    }

    #[test]
    fn test_approve_all() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter.clone(), &[]);
        let token_id = String::default();
        let operator = "operator";
        let operator_info = mock_info(operator.clone(), &[]);
        //store config
        CONFIG
            .save(
                deps.as_mut().storage,
                &TokenConfig {
                    name: TOKEN_NAME.to_string(),
                    symbol: TOKEN_SYMBOL.to_string(),
                    minter: String::from("creator"),
                    metadata_limit: None,
                },
            )
            .unwrap();

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
            metadata: None,
        });
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

        assert_eq!(
            err,
            StdError::generic_err("Address does not have transfer rights for this token"),
        );

        let approve_all_msg = ExecuteMsg::ApproveAll {
            operator: operator.to_string(),
            expires: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), approve_all_msg).unwrap();

        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: operator.to_string(),
            token_id: token_id.clone(),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            operator_info.clone(),
            transfer_msg,
        )
        .unwrap();

        let token = TOKENS
            .load(deps.as_ref().storage, token_id.to_string())
            .unwrap();

        assert_eq!(token.owner, operator.to_string());
    }

    #[test]
    fn test_revoke_all() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter.clone(), &[]);
        let token_id = String::default();
        let operator = "operator";
        let operator_info = mock_info(operator.clone(), &[]);

        //store config
        CONFIG
            .save(
                deps.as_mut().storage,
                &TokenConfig {
                    name: TOKEN_NAME.to_string(),
                    symbol: TOKEN_SYMBOL.to_string(),
                    minter: String::from("creator"),
                    metadata_limit: None,
                },
            )
            .unwrap();

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
            metadata: None,
        });
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
        execute(deps.as_mut(), env.clone(), info.clone(), revoke_msg).unwrap();

        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: minter.to_string(),
            token_id: token_id.clone(),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            operator_info.clone(),
            transfer_msg,
        )
        .unwrap_err();

        assert_eq!(
            err,
            StdError::generic_err("Address does not have transfer rights for this token"),
        );
    }

    #[test]
    fn test_transfer_agreement() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let purchaser = "purchaser";
        let info = mock_info(minter.clone(), &[]);
        let token_id = String::default();
        let denom = "uluna";
        let amount = Uint128::from(100 as u64);

        let instantiate_msg = InstantiateMsg {
            name: "Token Name".to_string(),
            symbol: "TS".to_string(),
            minter: minter.to_string(),
            init_hook: None,
            metadata_limit: None,
            modules: vec![],
            receipt_code_id: 1,
            address_list_code_id: Some(2),
        };
        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
            metadata: None,
        });
        execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

        let transfer_agreement_msg = ExecuteMsg::TransferAgreement {
            token_id: token_id.clone(),
            denom: denom.to_string(),
            amount: amount.clone(),
            purchaser: purchaser.to_string(),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            transfer_agreement_msg,
        )
        .unwrap();

        let agreement_query = QueryMsg::NftTransferAgreementInfo {
            token_id: token_id.clone(),
        };
        let res = query(deps.as_ref(), env.clone(), agreement_query).unwrap();
        let agreement_res: NftTransferAgreementResponse = from_binary(&res).unwrap();
        let agreement = agreement_res.agreement.unwrap();

        assert_eq!(agreement.purchaser, purchaser.clone());
        assert_eq!(agreement.amount, coin(amount.u128(), denom));

        let purchaser_info = mock_info(purchaser.clone(), &[]);
        let transfer_msg = ExecuteMsg::TransferNft {
            token_id: token_id.clone(),
            recipient: purchaser.to_string(),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            purchaser_info.clone(),
            transfer_msg,
        )
        .unwrap();
    }

    #[test]
    fn test_metadata() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter.clone(), &[]);
        let token_id = "1";

        let instantiate_message = InstantiateMsg {
            name: "Token".to_string(),
            symbol: "T".to_string(),
            minter: minter.to_string(),
            modules: vec![],
            receipt_code_id: RECEIPT_CODE_ID,
            init_hook: None,
            metadata_limit: Some(4),
            address_list_code_id: Some(ADDRESS_LIST_CODE_ID),
        };

        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            instantiate_message,
        )
        .unwrap();

        let metadata = "really long metadata message, too long for the storage".to_string();

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            name: "test token".to_string(),
            description: None,
            metadata: Some(metadata.clone()),
        });

        let res = execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap_err();

        assert_eq!(
            res,
            StdError::generic_err("Metadata length must be less than or equal to 4")
        );

        let metadata = "s".to_string();

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            name: "test token".to_string(),
            description: None,
            metadata: Some(metadata.clone()),
        });

        let res = execute(deps.as_mut(), env.clone(), info.clone(), mint_msg).unwrap();

        assert_eq!(res, Response::default());

        let query_msg = QueryMsg::NftMetadata {
            token_id: token_id.to_string(),
        };

        let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let query_val: NftMetadataResponse = from_binary(&query_res).unwrap();

        assert_eq!(query_val.metadata, Some(metadata.clone()))
    }

    #[test]
    fn test_execute_burn() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter.clone(), &[]);
        let token_id = "1";

        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
        };

        let msg = ExecuteMsg::Mint(mint_msg);

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let unauth_info = mock_info("anyone", &[]);
        let burn_msg = ExecuteMsg::Burn {
            token_id: token_id.to_string(),
        };

        let resp = execute(deps.as_mut(), env.clone(), unauth_info, burn_msg.clone()).unwrap_err();

        assert_eq!(
            resp,
            StdError::generic_err("Cannot burn a token you do not own")
        );

        execute(deps.as_mut(), env.clone(), info.clone(), burn_msg.clone()).unwrap();

        let query_msg = QueryMsg::OwnerOf {
            token_id: token_id.to_string(),
        };

        let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap_err();

        assert_eq!(
            query_res,
            StdError::not_found("andromeda_protocol::token::Token")
        )
    }

    #[test]
    fn test_execute_archive() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let info = mock_info(minter.clone(), &[]);
        let token_id = "1";

        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: minter.to_string(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
            metadata: None,
        };

        let msg = ExecuteMsg::Mint(mint_msg);

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let unauth_info = mock_info("anyone", &[]);
        let archive_msg = ExecuteMsg::Archive {
            token_id: token_id.to_string(),
        };

        let resp =
            execute(deps.as_mut(), env.clone(), unauth_info, archive_msg.clone()).unwrap_err();

        assert_eq!(
            resp,
            StdError::generic_err("Cannot archive a token you do not own")
        );

        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            archive_msg.clone(),
        )
        .unwrap();

        let archived_resp = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            archive_msg.clone(),
        )
        .unwrap_err();
        assert_eq!(
            archived_resp,
            StdError::generic_err("This token is archived and cannot be changed in any way.")
        );

        let query_msg = QueryMsg::NftArchiveStatus {
            token_id: token_id.to_string(),
        };

        let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let query_val: NftArchivedResponse = from_binary(&query_res).unwrap();
        assert!(query_val.archived)
    }
}
