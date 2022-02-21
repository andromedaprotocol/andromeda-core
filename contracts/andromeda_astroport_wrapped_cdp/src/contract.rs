use crate::state::{Config, CONFIG};
use andromeda_protocol::{
    astroport_wrapped_cdp::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    communication::encode_binary,
    error::ContractError,
    ownership::{execute_update_owner, is_contract_owner, query_contract_owner, CONTRACT_OWNER},
    require,
    swapper::{AssetInfo, SwapperMsg},
};
use astroport::{
    asset::{AssetInfo as AstroportAssetInfo, PairInfo},
    factory::QueryMsg as AstroportFactoryQueryMsg,
    router::{ExecuteMsg as AstroportRouterExecuteMsg, SwapOperation},
};
use cosmwasm_std::{
    attr, entry_point, from_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};

use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use andromeda_protocol::astroport_wrapped_cdp::{ConfigResponse, Cw20HookMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_astroport_wrapped_cdp";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        astroport_factory_contract: deps.api.addr_validate(&msg.astroport_factory_contract)?,
        astroport_router_contract: deps.api.addr_validate(&msg.astroport_router_contract)?,
        astroport_staking_contract: deps.api.addr_validate(&msg.astroport_staking_contract)?,
        astroport_vesting_contract: deps.api.addr_validate(&msg.astroport_vesting_contract)?,
        astroport_maker_contract: deps.api.addr_validate(&msg.astroport_maker_contract)?,
    };

    CONFIG.save(deps.storage, &config)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "instantiate"),
        attr("type", "astroport"),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    match msg {
        ExecuteMsg::Swapper(msg) => handle_swapper_msg(deps, info, msg),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::AstroportFactoryExecuteMsg(msg) => execute_astroport_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.astroport_factory_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::AstroportRouterExecuteMsg(msg) => execute_astroport_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.astroport_router_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::AstroportStakingExecuteMsg(msg) => execute_astroport_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.astroport_staking_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::AstroportVestingExecuteMsg(msg) => execute_astroport_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.astroport_vesting_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::AstroportMakerExecuteMsg(msg) => execute_astroport_msg(
            deps,
            info.sender.to_string(),
            info.funds,
            config.astroport_maker_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::UpdateOwner { address } => execute_update_owner(deps, info, address),
        ExecuteMsg::UpdateConfig {
            astroport_factory_contract,
            astroport_router_contract,
            astroport_staking_contract,
            astroport_vesting_contract,
            astroport_maker_contract,
        } => execute_update_config(
            deps,
            info,
            astroport_factory_contract,
            astroport_router_contract,
            astroport_staking_contract,
            astroport_vesting_contract,
            astroport_maker_contract,
        ),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_address = info.sender.to_string();
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::AstroportRouterCw20HookMsg(msg) => execute_astroport_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            config.astroport_router_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::AstroportStakingCw20HookMsg(msg) => execute_astroport_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            config.astroport_staking_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::AstroportVestingCw20HookMsg(msg) => execute_astroport_cw20_msg(
            deps,
            cw20_msg.sender,
            token_address,
            cw20_msg.amount,
            config.astroport_vesting_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::Swapper(msg) => handle_swapper_msg_cw20(
            deps,
            cw20_msg.sender,
            msg,
            info.sender.to_string(),
            cw20_msg.amount,
        ),
    }
}

fn handle_swapper_msg(
    deps: DepsMut,
    info: MessageInfo,
    msg: SwapperMsg,
) -> Result<Response, ContractError> {
    match msg {
        SwapperMsg::Swap {
            offer_asset_info,
            ask_asset_info,
        } => execute_swap(deps, info, offer_asset_info, ask_asset_info),
    }
}

fn handle_swapper_msg_cw20(
    deps: DepsMut,
    sender: String,
    msg: SwapperMsg,
    token_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    match msg {
        SwapperMsg::Swap {
            offer_asset_info,
            ask_asset_info,
        } => execute_swap_cw20(
            deps,
            sender,
            token_addr,
            amount,
            offer_asset_info,
            ask_asset_info,
        ),
    }
}

fn execute_swap(
    deps: DepsMut,
    info: MessageInfo,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let operations: Vec<SwapOperation> = get_swap_operations(
        &deps.querier,
        offer_asset_info,
        ask_asset_info,
        config.astroport_factory_contract.to_string(),
    )?;
    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        to: Some(info.sender.clone()),
    };

    execute_astroport_msg(
        deps,
        info.sender.to_string(),
        info.funds,
        config.astroport_router_contract.to_string(),
        encode_binary(&swap_msg)?,
    )
}

fn execute_swap_cw20(
    deps: DepsMut,
    sender: String,
    token_addr: String,
    amount: Uint128,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let operations: Vec<SwapOperation> = get_swap_operations(
        &deps.querier,
        offer_asset_info,
        ask_asset_info,
        config.astroport_factory_contract.to_string(),
    )?;
    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        to: Some(deps.api.addr_validate(&sender)?),
    };

    execute_astroport_cw20_msg(
        deps,
        sender,
        token_addr,
        amount,
        config.astroport_router_contract.to_string(),
        encode_binary(&swap_msg)?,
    )
}

fn get_swap_operations(
    querier: &QuerierWrapper,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
    factory_address: String,
) -> Result<Vec<SwapOperation>, ContractError> {
    let existing_pair = get_pair(
        &querier,
        offer_asset_info.clone(),
        ask_asset_info.clone(),
        factory_address,
    );
    Ok(if existing_pair.is_ok() {
        vec![SwapOperation::AstroSwap {
            offer_asset_info: offer_asset_info.into(),
            ask_asset_info: ask_asset_info.into(),
        }]
    } else if let [AssetInfo::NativeToken { denom: offer_denom }, AssetInfo::NativeToken { denom: ask_denom }] =
        [offer_asset_info.clone(), ask_asset_info.clone()]
    {
        vec![SwapOperation::NativeSwap {
            offer_denom,
            ask_denom,
        }]
    } else {
        // Use uusd as an intermediary (it is very likely that each asset has a uusd pool).
        vec![
            SwapOperation::AstroSwap {
                offer_asset_info: offer_asset_info.into(),
                ask_asset_info: AstroportAssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AstroportAssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                ask_asset_info: ask_asset_info.into(),
            },
        ]
    })
}

fn get_pair(
    querier: &QuerierWrapper,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
    factory_address: String,
) -> Result<PairInfo, ContractError> {
    Ok(
        querier.query::<PairInfo>(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: factory_address,
            msg: encode_binary(&AstroportFactoryQueryMsg::Pair {
                asset_infos: [
                    offer_asset_info.clone().into(),
                    ask_asset_info.clone().into(),
                ],
            })?,
        }))?,
    )
}

pub fn execute_astroport_cw20_msg(
    deps: DepsMut,
    sender: String,
    token_addr: String,
    amount: Uint128,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    let msg = Cw20ExecuteMsg::Send {
        contract: contract_addr,
        amount,
        msg: msg_binary,
    };
    execute_astroport_msg(deps, sender, vec![], token_addr, encode_binary(&msg)?)
}

pub fn execute_astroport_msg(
    deps: DepsMut,
    _sender: String,
    funds: Vec<Coin>,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    require(
        funds.is_empty() || funds.len() == 1,
        ContractError::InvalidAstroportFunds {
            msg: "Astroport expects no funds or a single type of fund to be deposited.".to_string(),
        },
    )?;

    let execute_msg = WasmMsg::Execute {
        contract_addr,
        funds,
        msg: msg_binary,
    };

    Ok(Response::new().add_message(CosmosMsg::Wasm(execute_msg)))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    astroport_factory_contract: Option<String>,
    astroport_router_contract: Option<String>,
    astroport_staking_contract: Option<String>,
    astroport_vesting_contract: Option<String>,
    astroport_maker_contract: Option<String>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {},
    )?;
    let mut config = CONFIG.load(deps.storage)?;
    if let Some(astroport_factory_contract) = astroport_factory_contract {
        config.astroport_factory_contract = deps.api.addr_validate(&astroport_factory_contract)?;
    }
    if let Some(astroport_router_contract) = astroport_router_contract {
        config.astroport_router_contract = deps.api.addr_validate(&astroport_router_contract)?;
    }
    if let Some(astroport_staking_contract) = astroport_staking_contract {
        config.astroport_staking_contract = deps.api.addr_validate(&astroport_staking_contract)?;
    }
    if let Some(astroport_vesting_contract) = astroport_vesting_contract {
        config.astroport_vesting_contract = deps.api.addr_validate(&astroport_vesting_contract)?;
    }
    if let Some(astroport_maker_contract) = astroport_maker_contract {
        config.astroport_vesting_contract = deps.api.addr_validate(&astroport_maker_contract)?;
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::ContractOwner {} => encode_binary(&query_contract_owner(deps)?),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        astroport_factory_contract: config.astroport_factory_contract.to_string(),
        astroport_router_contract: config.astroport_router_contract.to_string(),
        astroport_staking_contract: config.astroport_staking_contract.to_string(),
        astroport_vesting_contract: config.astroport_vesting_contract.to_string(),
        astroport_maker_contract: config.astroport_maker_contract.to_string(),
    })
}
