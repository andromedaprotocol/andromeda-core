use andromeda_modules::modules::{read_modules, MODULES};
use andromeda_protocol::token::{
    ExecuteMsg, InstantiateMsg, MintMsg, OwnerResponse, QueryMsg, TokenId,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::state::{TokenConfig, CONFIG, OWNERSHIP};

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

    OWNERSHIP.save(
        deps.storage,
        msg.token_id.to_string(),
        &info.sender.to_string(),
    )?;

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

    OWNERSHIP.save(
        deps.storage,
        msg.token_id.to_string(),
        &info.sender.to_string(),
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner { token_id } => to_binary(&query_owner(deps, token_id)?),
    }
}

fn query_owner(deps: Deps, token_id: TokenId) -> StdResult<OwnerResponse> {
    let owner = OWNERSHIP.load(deps.storage, token_id.to_string())?;
    Ok(OwnerResponse { owner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    const TOKEN_NAME: &str = "test";
    const TOKEN_SYMBOL: &str = "T";

    #[test]
    fn proper_initialization() {
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
}
