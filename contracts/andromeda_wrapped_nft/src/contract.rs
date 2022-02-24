use crate::state::{Config, CONFIG, CUR_TOKEN_ID, TOKENIDS};
use andromeda_protocol::error::ContractError;
use andromeda_protocol::wrapped_nft::{MigrateMsg, TokenInfoResponse};
use andromeda_protocol::{
    factory::{AddressResponse, ExecuteMsg as FactoryExecuteMsg, QueryMsg as FactoryQueryMsg},
    token::{ExecuteMsg as TokenExecuteMsg, MintMsg as TokenMintMsg},
    wrapped_nft::{ConfigResponse, Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg},
};
use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut,
    Env, MessageInfo, QueryRequest, Reply, ReplyOn, Response, StdError, StdResult, SubMsg, WasmMsg,
    WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw721::Cw721ReceiveMsg;

pub const REPLY_CREATE_TOKEN: u64 = 1;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-wrapped-nft";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        factory_addr: deps
            .api
            .addr_canonicalize(msg.andromeda_factory_addr.as_str())?,
        token_addr: CanonicalAddr::from(vec![]),
    };

    CONFIG.save(deps.storage, &config)?;
    CUR_TOKEN_ID.save(deps.storage, &0u64)?;

    let create_token_msg = FactoryExecuteMsg::Create {
        name: msg.name,
        symbol: msg.symbol,
        modules: msg.modules,
    };

    let create_msg = WasmMsg::Execute {
        contract_addr: msg.andromeda_factory_addr.to_string(),
        msg: to_binary(&create_token_msg)?,
        funds: vec![],
    };

    let msg = SubMsg {
        msg: create_msg.into(),
        gas_limit: None,
        id: REPLY_CREATE_TOKEN,
        reply_on: ReplyOn::Always,
    };

    Ok(Response::new()
        .add_submessage(msg)
        .add_attributes(vec![attr("action", "instantiate")]))
}
#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    if msg.result.is_err() {
        return Err(StdError::generic_err(msg.result.unwrap_err()));
    }

    match msg.id {
        REPLY_CREATE_TOKEN => on_token_creation_reply(deps, msg),
        _ => Err(StdError::generic_err("reply id is invalid")),
    }
}

fn on_token_creation_reply(deps: DepsMut, _msg: Reply) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;

    let res: AddressResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: deps.api.addr_humanize(&config.factory_addr)?.to_string(),
        msg: to_binary(&FactoryQueryMsg::GetAddress {
            symbol: config.symbol.clone(),
        })?,
    }))?;
    config.token_addr = deps.api.addr_canonicalize(&res.address)?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "reply")]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::ReceiveNft(cw721_msg) => receive_cw721(deps, info, cw721_msg),
    }
}

fn receive_cw721(
    deps: DepsMut,
    info: MessageInfo,
    cw721_msg: Cw721ReceiveMsg,
) -> StdResult<Response> {
    let nft_addr = info.sender.to_string();
    match from_binary(&cw721_msg.msg)? {
        Cw721HookMsg::Wrap {} => {
            execute_wrap_nft(deps, cw721_msg.sender, nft_addr, cw721_msg.token_id)
        }
        Cw721HookMsg::Unwrap {} => {
            execute_unwrap_nft(deps, cw721_msg.sender, nft_addr, cw721_msg.token_id)
        }
    }
}
pub fn execute_wrap_nft(
    deps: DepsMut,
    sender: String,
    nft_addr: String,
    token_id: String,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let cur_token_id = CUR_TOKEN_ID.load(deps.storage)?;
    let create_mint_msg = TokenExecuteMsg::Mint(TokenMintMsg {
        token_id: cur_token_id.to_string(),
        owner: sender,
        name: config.name,
        token_uri: None,
        description: None,
        metadata: None,
        pricing: None,
    });

    TOKENIDS.save(
        deps.storage,
        cur_token_id.to_string(),
        &(token_id, nft_addr),
    )?;
    CUR_TOKEN_ID.save(deps.storage, &(cur_token_id + 1))?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&config.token_addr)?.to_string(),
            msg: to_binary(&create_mint_msg)?,
            funds: vec![],
        }))
        .add_attributes(vec![
            attr("action", "mint_wrap_nft"),
            attr("new_token_id", cur_token_id.to_string()),
        ]))
}
pub fn execute_unwrap_nft(
    deps: DepsMut,
    sender: String,
    nft_addr: String,
    token_id: String,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if config.token_addr != deps.api.addr_canonicalize(&nft_addr)? {
        return Err(StdError::generic_err("nft contract is not this contract"));
    }
    let (original_token_id, nft_addr) = TOKENIDS.load(deps.storage, token_id.clone())?;
    TOKENIDS.remove(deps.storage, token_id.clone());

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.token_addr)?.to_string(),
                msg: to_binary(&TokenExecuteMsg::Burn {
                    token_id: token_id.clone(),
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: nft_addr,
                msg: to_binary(&TokenExecuteMsg::TransferNft {
                    recipient: sender.clone(),
                    token_id: original_token_id.clone(),
                })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![
            attr("action", "unwrap_nft"),
            attr("burn_token_id", token_id),
            attr("receiver", sender.clone()),
            attr("token_id", original_token_id.clone()),
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::TokenInfo { token_id } => to_binary(&query_token_info(deps, token_id)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        name: config.name,
        symbol: config.symbol,
        factory_addr: deps.api.addr_humanize(&config.factory_addr)?.to_string(),
        token_addr: deps.api.addr_humanize(&config.factory_addr)?.to_string(),
    })
}

fn query_token_info(deps: Deps, token_id: String) -> StdResult<TokenInfoResponse> {
    let token_info = TOKENIDS.load(deps.storage, token_id)?;
    Ok(TokenInfoResponse {
        original_token_id: token_info.0,
        original_token_addr: token_info.1,
    })
}
