use crate::state::SWAPPER_IMPL_ADDR;
use andromeda_protocol::{
    communication::{encode_binary, modules::InstantiateType, query_get, Recipient},
    error::ContractError,
    factory::CodeIdResponse,
    ownership::CONTRACT_OWNER,
    require,
    response::get_reply_address,
    swapper::{
        query_balance, query_token_balance, AssetInfo, Cw20HookMsg, ExecuteMsg, InstantiateMsg,
        MigrateMsg, QueryMsg, SwapperCw20HookMsg, SwapperImplCw20HookMsg, SwapperImplExecuteMsg,
        SwapperMsg,
    },
};
use cosmwasm_std::{
    entry_point, from_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdError, SubMsg, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_swapper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;

    let mut resp: Response = Response::new();
    match msg.swapper_impl {
        InstantiateType::Address(addr) => SWAPPER_IMPL_ADDR.save(deps.storage, &addr)?,
        InstantiateType::New(instantiate_msg) => {
            let code_id: u64 = query_get::<CodeIdResponse>(
                Some(encode_binary(&"cw721")?),
                // TODO: Replace when Primitive contract change merged.
                "TEMP_FACTORY".to_string(),
                deps.querier,
            )?
            .code_id;
            let msg: SubMsg = SubMsg {
                id: 1,
                reply_on: ReplyOn::Always,
                msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: None,
                    code_id,
                    msg: encode_binary(&instantiate_msg)?,
                    funds: vec![],
                    label: "Instantiate: swapper implementation".to_string(),
                }),
                gas_limit: None,
            };
            resp = resp.add_submessage(msg);
        }
    }

    Ok(resp.add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }
    require(msg.id == 1, ContractError::InvalidReplyId {})?;

    let addr = get_reply_address(&msg)?;
    SWAPPER_IMPL_ADDR.save(deps.storage, &addr)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Swap {
            ask_asset_info,
            recipient,
        } => execute_swap(deps, env, info, ask_asset_info, recipient),
        ExecuteMsg::Send {
            ask_asset_info,
            recipient,
        } => execute_send(deps, env, info, ask_asset_info, recipient),
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ask_asset_info: AssetInfo,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let recipient = recipient.unwrap_or_else(|| Recipient::Addr(info.sender.to_string()));
    require(
        info.funds.len() <= 1,
        ContractError::InvalidFunds {
            msg: "Must send at most one native coin".to_string(),
        },
    )?;
    if info.funds.is_empty() || info.funds[0].amount.is_zero() {
        return Ok(Response::new());
    }
    let coin = &info.funds[0];
    if let AssetInfo::NativeToken { denom } = &ask_asset_info {
        if denom == &coin.denom {
            // Send coins as is as there is no need to swap.
            let msg = recipient.generate_msg_native(deps.api, info.funds)?;
            return Ok(Response::new()
                .add_submessage(msg)
                .add_attribute("action", "swap"));
        }
    }
    let contract_addr = SWAPPER_IMPL_ADDR.load(deps.storage)?;
    let denom = coin.denom.clone();
    Ok(Response::new()
        .add_attribute("action", "swap")
        .add_attribute("offer_denom", &denom)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds: info.funds,
            msg: encode_binary(&SwapperImplExecuteMsg::Swapper(SwapperMsg::Swap {
                offer_asset_info: AssetInfo::NativeToken { denom },
                ask_asset_info: ask_asset_info.clone(),
            }))?,
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: encode_binary(&ExecuteMsg::Send {
                ask_asset_info,
                recipient,
            })?,
        })))
}

fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ask_asset_info: AssetInfo,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    require(
        info.sender == env.contract.address,
        ContractError::Unauthorized {},
    )?;
    let msg: SubMsg = match ask_asset_info {
        AssetInfo::NativeToken { denom } => {
            let amount = query_balance(&deps.querier, env.contract.address, denom.clone())?;
            recipient.generate_msg_native(deps.api, vec![Coin { denom, amount }])?
        }
        AssetInfo::Token { contract_addr } => {
            let amount =
                query_token_balance(&deps.querier, contract_addr.clone(), env.contract.address)?;
            recipient.generate_msg_cw20(
                deps.api,
                Cw20Coin {
                    address: contract_addr.to_string(),
                    amount,
                },
            )?
        }
    };
    Ok(Response::new()
        .add_attribute("action", "send")
        .add_submessage(msg))
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::Swap {
            ask_asset_info,
            recipient,
        } => execute_swap_cw20(
            deps,
            env,
            info.sender.to_string(),
            cw20_msg.amount,
            ask_asset_info,
            cw20_msg.sender,
            recipient,
        ),
    }
}

fn execute_swap_cw20(
    deps: DepsMut,
    env: Env,
    offer_token: String,
    offer_amount: Uint128,
    ask_asset_info: AssetInfo,
    sender: String,
    recipient: Option<Recipient>,
) -> Result<Response, ContractError> {
    let recipient = recipient.unwrap_or(Recipient::Addr(sender));
    if let AssetInfo::Token { contract_addr } = &ask_asset_info {
        if *contract_addr == offer_token {
            // Send as is.
            let msg = recipient.generate_msg_cw20(
                deps.api,
                Cw20Coin {
                    address: offer_token,
                    amount: offer_amount,
                },
            )?;
            return Ok(Response::new()
                .add_submessage(msg)
                .add_attribute("action", "swap"));
        }
    }
    let contract_addr = SWAPPER_IMPL_ADDR.load(deps.storage)?;
    Ok(Response::new()
        .add_attribute("action", "swap")
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: offer_token,
            funds: vec![],
            msg: encode_binary(&Cw20ExecuteMsg::Send {
                contract: contract_addr,
                amount: offer_amount,
                msg: encode_binary(&SwapperImplCw20HookMsg::Swapper(SwapperCw20HookMsg::Swap {
                    ask_asset_info: ask_asset_info.clone(),
                }))?,
            })?,
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: encode_binary(&ExecuteMsg::Send {
                ask_asset_info,
                recipient,
            })?,
        })))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> Result<Binary, ContractError> {
    Err(ContractError::UnsupportedOperation {})
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
