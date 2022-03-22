use crate::state::{Config, CONFIG};
use ado_base::state::ADOContract;
use andromeda_protocol::{
    astroport::{ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    swapper::{AssetInfo, SwapperCw20HookMsg, SwapperMsg},
};
use astroport::{
    asset::AssetInfo as AstroportAssetInfo,
    querier::query_pair_info,
    router::{
        Cw20HookMsg as AstroportRouterCw20HookMsg, ExecuteMsg as AstroportRouterExecuteMsg,
        SwapOperation,
    },
};
use common::{
    ado_base::InstantiateMsg as BaseInstantiateMsg, encode_binary, error::ContractError, require,
};
use cosmwasm_std::{
    entry_point, from_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, Response, StdResult, Uint128, WasmMsg,
};

use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_astroport";
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
    };

    CONFIG.save(deps.storage, &config)?;
    ADOContract::default().instantiate(
        deps.storage,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: "astroport".to_string(),
            operators: None,
            modules: None,
            primitive_contract: None,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::Swapper(msg) => handle_swapper_msg(deps, info, msg),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::AstroportFactoryExecuteMsg(msg) => execute_astroport_msg(
            info.funds,
            config.astroport_factory_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::AstroportRouterExecuteMsg(msg) => execute_astroport_msg(
            info.funds,
            config.astroport_router_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::AstroportStakingExecuteMsg(msg) => execute_astroport_msg(
            info.funds,
            config.astroport_staking_contract.to_string(),
            encode_binary(&msg)?,
        ),
        ExecuteMsg::UpdateConfig {
            astroport_factory_contract,
            astroport_router_contract,
            astroport_staking_contract,
        } => execute_update_config(
            deps,
            info,
            astroport_factory_contract,
            astroport_router_contract,
            astroport_staking_contract,
        ),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_address = info.sender;
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::AstroportRouterCw20HookMsg(msg) => execute_astroport_cw20_msg(
            token_address.to_string(),
            cw20_msg.amount,
            config.astroport_router_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::AstroportStakingCw20HookMsg(msg) => execute_astroport_cw20_msg(
            token_address.to_string(),
            cw20_msg.amount,
            config.astroport_staking_contract.to_string(),
            encode_binary(&msg)?,
        ),
        Cw20HookMsg::Swapper(msg) => {
            handle_swapper_msg_cw20(deps, cw20_msg.sender, msg, token_address, cw20_msg.amount)
        }
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
    msg: SwapperCw20HookMsg,
    token_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    match msg {
        SwapperCw20HookMsg::Swap { ask_asset_info } => execute_swap_cw20(
            deps,
            sender,
            token_addr.to_string(),
            amount,
            AssetInfo::Token {
                contract_addr: token_addr,
            },
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
        config.astroport_factory_contract,
    )?;
    let swap_msg = AstroportRouterExecuteMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        to: Some(info.sender.clone()),
    };

    execute_astroport_msg(
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
        config.astroport_factory_contract,
    )?;
    let swap_msg = AstroportRouterCw20HookMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        to: Some(sender),
    };

    execute_astroport_cw20_msg(
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
    factory_address: Addr,
) -> Result<Vec<SwapOperation>, ContractError> {
    let existing_pair = query_pair_info(
        querier,
        factory_address,
        &[
            offer_asset_info.clone().into(),
            ask_asset_info.clone().into(),
        ],
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
        let first_swap = if let AssetInfo::NativeToken { denom } = offer_asset_info {
            SwapOperation::NativeSwap {
                offer_denom: denom,
                ask_denom: "uusd".to_string(),
            }
        } else {
            SwapOperation::AstroSwap {
                offer_asset_info: offer_asset_info.into(),
                ask_asset_info: AstroportAssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            }
        };
        vec![
            first_swap,
            SwapOperation::AstroSwap {
                offer_asset_info: AstroportAssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                ask_asset_info: ask_asset_info.into(),
            },
        ]
    })
}

pub fn execute_astroport_cw20_msg(
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
    execute_astroport_msg(vec![], token_addr, encode_binary(&msg)?)
}

pub fn execute_astroport_msg(
    funds: Vec<Coin>,
    contract_addr: String,
    msg_binary: Binary,
) -> Result<Response, ContractError> {
    require(
        funds.is_empty() || funds.len() == 1,
        ContractError::InvalidFunds {
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
) -> Result<Response, ContractError> {
    require(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        astroport_factory_contract: config.astroport_factory_contract.to_string(),
        astroport_router_contract: config.astroport_router_contract.to_string(),
        astroport_staking_contract: config.astroport_staking_contract.to_string(),
    })
}
