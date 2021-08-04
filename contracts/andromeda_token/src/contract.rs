use andromeda_modules::{
    common::require,
    modules::{read_modules, MODULES},
};
use andromeda_protocol::token::{
    Approval, ExecuteMsg, InstantiateMsg, MintMsg, OwnerResponse, QueryMsg, Token, TokenId,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw721::{Cw721ReceiveMsg, Expiration};

use crate::state::{has_transfer_rights, TokenConfig, CONFIG, OPERATOR, TOKENS};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    msg.validate()?;

    let config = TokenConfig {
        name: msg.name,
        symbol: msg.symbol,
        minter: msg.minter,
    };

    CONFIG.save(deps.storage, &config)?;
    MODULES.save(deps.storage, &msg.modules)?;

    match msg.init_hook {
        Some(hook) => {
            let resp = Response::new().add_message(hook.into_cosmos_msg()?);
            Ok(resp)
        }
        None => Ok(Response::default()),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    for module in modules {
        module.pre_execute(&deps, info.clone(), env.clone())?;
    }

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
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MintMsg,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    for module in modules {
        module.pre_publish(&deps, env.clone(), msg.token_id.clone())?;
    }

    let token = Token {
        token_id: msg.token_id.clone(),
        owner: info.sender.to_string(),
        description: msg.description,
        name: msg.name,
        approvals: vec![],
    };

    TOKENS.save(deps.storage, msg.token_id.to_string(), &token)?;

    Ok(Response::default())
}

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: i64,
) -> StdResult<Response> {
    let modules = read_modules(deps.storage)?;
    for module in modules {
        module.pre_transfer(&deps, env.clone(), recipient.clone(), token_id.clone())?;
    }

    transfer_nft(deps, &env, &info, &recipient, &token_id)?;

    Ok(Response::default())
}

pub fn execute_send_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    token_id: i64,
    msg: Binary,
) -> StdResult<Response> {
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
    _env: Env,
    info: MessageInfo,
    token_id: i64,
    spender: String,
    expires: Option<Expiration>,
) -> StdResult<Response> {
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
    _env: Env,
    info: MessageInfo,
    token_id: i64,
    spender: String,
) -> StdResult<Response> {
    let spender_addr = deps.api.addr_validate(&spender)?;

    remove_approval(deps, &info, token_id, &spender_addr)?;

    Ok(Response::default())
}

fn execute_approve_all(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    operator: String,
    expires: Option<Expiration>,
) -> StdResult<Response> {
    OPERATOR.save(
        deps.storage,
        (info.sender.to_string(), operator.clone()),
        &expires.unwrap_or_default(),
    )?;

    Ok(Response::default())
}

fn execute_revoke_all(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    operator: String,
) -> StdResult<Response> {
    OPERATOR.remove(deps.storage, (info.sender.to_string(), operator.clone()));

    Ok(Response::default())
}

fn transfer_nft(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    recipient: &String,
    token_id: &i64,
) -> StdResult<()> {
    let mut token = TOKENS.load(deps.storage, token_id.to_string())?;
    require(
        has_transfer_rights(deps.storage, env, info.sender.to_string(), &token)?,
        StdError::generic_err("Address does not have transfer rights for this token"),
    )?;

    token.owner = recipient.to_string();
    token.approvals = vec![];

    TOKENS.save(deps.storage, token_id.to_string(), &token)?;
    Ok(())
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

    token.filter_approval(spender);

    TOKENS.save(deps.storage, token_id.to_string(), &token)?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner { token_id } => to_binary(&query_owner(deps, token_id)?),
    }
}

fn query_owner(deps: Deps, token_id: TokenId) -> StdResult<OwnerResponse> {
    let owner = TOKENS.load(deps.storage, token_id.to_string())?.owner;
    Ok(OwnerResponse { owner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::token::Approval;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Api};

    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "T";

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {
            name: TOKEN_NAME.to_string(),
            symbol: TOKEN_SYMBOL.to_string(),
            modules: vec![],
            minter: String::from("creator"),
            init_hook: None,
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
        let token_id = 1;
        let creator = "creator".to_string();

        let mint_msg = MintMsg {
            token_id,
            owner: creator.clone(),
            description: Some("Test Token".to_string()),
            name: "TestToken".to_string(),
        };

        let msg = ExecuteMsg::Mint(mint_msg);

        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let query_msg = QueryMsg::GetOwner { token_id };

        let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let query_val: OwnerResponse = from_binary(&query_res).unwrap();

        assert_eq!(query_val.owner, creator)
    }

    #[test]
    fn test_transfer() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let minter = "minter";
        let recipient = "recipient";
        let info = mock_info(minter.clone(), &[]);
        let token_id = 1;
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient.to_string(),
            token_id: token_id.clone(),
        };

        let token = Token {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: String::default(),
            approvals: vec![],
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
            token_id: 2,
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
        assert_eq!(Response::default(), res);
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
        let approval_token_id = 2;
        let approval_token = Token {
            token_id: approval_token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: String::default(),
            approvals: vec![approval],
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
        assert_eq!(Response::default(), res);
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
        let approval_token_id = 2;
        let approval_token = Token {
            token_id: approval_token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: String::default(),
            approvals: vec![approval],
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
        assert_eq!(Response::default(), res);
        let owner = TOKENS
            .load(deps.as_ref().storage, approval_token_id.to_string())
            .unwrap()
            .owner;
        assert_eq!(recipient.to_string(), owner);
    }

    #[test]
    fn test_approve() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let sender = "sender";
        let info = mock_info(sender.clone(), &[]);
        let token_id = 1;
        let approvee = "aprovee";

        let msg = ExecuteMsg::Approve {
            spender: approvee.to_string(),
            expires: None,
            token_id: 1,
        };

        let token = Token {
            token_id: token_id.clone(),
            description: None,
            name: String::default(),
            approvals: vec![],
            owner: sender.to_string(),
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
        let token_id = 1;
        let approvee = "aprovee";
        let approval = Approval {
            expires: Expiration::Never {},
            spender: deps.api.addr_validate(approvee.clone()).unwrap(),
        };

        let msg = ExecuteMsg::Revoke {
            spender: approvee.to_string(),
            token_id: 1,
        };

        let token = Token {
            token_id: token_id.clone(),
            description: None,
            name: String::default(),
            approvals: vec![approval],
            owner: sender.to_string(),
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
        let token_id = 1;
        let operator = "operator";
        let operator_info = mock_info(operator.clone(), &[]);

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
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
        let token_id = 1;
        let operator = "operator";
        let operator_info = mock_info(operator.clone(), &[]);

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: minter.to_string(),
            description: None,
            name: "Some Token".to_string(),
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
}
