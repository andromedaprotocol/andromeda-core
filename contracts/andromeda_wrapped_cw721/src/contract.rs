use crate::state::{ANDROMEDA_CW721_ADDR, CAN_UNWRAP, WRAPPED_TOKENS};
use andromeda_protocol::{
    communication::{encode_binary, query_get},
    cw721::{
        ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg, MetadataAttribute,
        MetadataType, TokenExtension, TokenMetadata,
    },
    error::ContractError,
    factory::CodeIdResponse,
    ownership::{is_contract_owner, CONTRACT_OWNER},
    require,
    response::get_reply_address,
    wrapped_cw721::{Cw721HookMsg, ExecuteMsg, InstantiateMsg, InstantiateType, QueryMsg},
};
use cosmwasm_std::{
    attr, coins, entry_point, from_binary, Addr, BankMsg, Binary, BlockInfo, Coin, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Reply, ReplyOn, Response, StdError,
    StdResult, Storage, SubMsg, Uint128, WasmMsg, WasmQuery, QueryRequest
};
use cw721::{Cw721QueryMsg, Cw721ReceiveMsg, NftInfoResponse};
use cw721_base::MintMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONTRACT_OWNER.save(deps.storage, &info.sender)?;
    CAN_UNWRAP.save(deps.storage, &msg.can_unwrap)?;
    let mut resp: Response = Response::new();
    match msg.cw721_instantiate_type {
        InstantiateType::Address(addr) => ANDROMEDA_CW721_ADDR.save(deps.storage, &addr)?,
        InstantiateType::New(specification) => {
            let instantiate_msg = Cw721InstantiateMsg {
                name: specification.name,
                symbol: specification.symbol,
                modules: specification.modules,
                minter: env.contract.address.to_string(),
            };
            let code_id: u64 = query_get::<CodeIdResponse>(
                Some(encode_binary(&"cw721")?),
                msg.factory_contract.to_string(),
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
                    label: "Instantiate: andromeda_cw721".to_string(),
                }),
                gas_limit: None,
            };
            resp = resp.add_submessage(msg);
        }
    }

    Ok(resp)
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
    ANDROMEDA_CW721_ADDR.save(deps.storage, &addr)?;
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
        ExecuteMsg::Receive(msg) => handle_receive_cw721(deps, env, info, msg),
    }
}

fn handle_receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&msg.msg)? {
        Cw721HookMsg::Wrap { wrapped_token_id } => execute_wrap(
            deps,
            env,
            msg.sender,
            msg.token_id,
            info.sender,
            wrapped_token_id,
        ),
        Cw721HookMsg::Unwrap {} => execute_unwrap(deps, env, msg.sender, msg.token_id, info.sender),
    }
}

fn execute_wrap(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_id: String,
    token_address: Addr,
    wrapped_token_id: Option<String>,
) -> Result<Response, ContractError> {
    require(
        is_contract_owner(deps.storage, &sender)?,
        ContractError::Unauthorized {},
    )?;
    require(
        token_address != env.contract.address,
        ContractError::CannotDoubleWrapToken {},
    )?;

    let extension = TokenExtension {
        name: token_id.clone(),
        publisher: sender.clone(),
        description: None,
        transfer_agreement: None,
        metadata: Some(TokenMetadata {
            data_type: MetadataType::Other,
            external_url: None,
            data_url: None,
            attributes: Some(vec![
                MetadataAttribute {
                    key: "original_token_id".to_string(),
                    value: token_id.clone(),
                    display_label: None,
                },
                MetadataAttribute {
                    key: "original_token_address".to_string(),
                    value: token_address.to_string(),
                    display_label: None,
                },
            ]),
        }),
        archived: false,
        pricing: None,
    };
    let wrapped_token_id = wrapped_token_id.unwrap_or_else(|| token_id.to_string());
    let mint_msg = MintMsg {
        token_id: wrapped_token_id.to_string(),
        owner: sender,
        token_uri: None,
        extension,
    };
    let msg = encode_binary(&Cw721ExecuteMsg::Mint(Box::new(mint_msg)))?;
    let cw721_contract_addr = ANDROMEDA_CW721_ADDR.load(deps.storage)?;
    let wasm_msg = WasmMsg::Execute {
        contract_addr: cw721_contract_addr,
        funds: vec![],
        msg,
    };
    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "wrap")
        .add_attribute("token_id", token_id)
        .add_attribute("token_address", token_address)
        .add_attribute("wrapped_token_id", wrapped_token_id))
}

fn execute_unwrap(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_id: String,
    token_address: Addr,
) -> Result<Response, ContractError> {
    let can_unwrap = CAN_UNWRAP.load(deps.storage)?;
    require(can_unwrap, ContractError::UnwrappingDisabled {})?;
    require(
        token_address == env.contract.address,
        ContractError::TokenNotWrappedByThisContract {},
    )?;
    let token_info = deps.querier.query(&QueryRequest::Wasm)
    let burn_msg = Cw721ExecuteMsg::Burn { token_id };
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    Ok(encode_binary(&"asdf".to_string())?)
}
