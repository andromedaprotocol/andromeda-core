use crate::state::{
    offers, query_is_archived, query_transfer_agreement, CW721_CONTRACT, VALID_DENOM,
};
use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::{
    cw721::QueryMsg as Cw721QueryMsg,
    cw721_offers::{
        AllOffersResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, Offer, OfferResponse, QueryMsg,
    },
};
use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        InstantiateMsg as BaseInstantiateMsg,
    },
    encode_binary,
    error::ContractError,
    rates::get_tax_amount,
    require, Funds,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    has_coins, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QuerierWrapper, QueryRequest, Response, StdError, Storage, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw721::{Expiration, OwnerOfResponse};
use cw_storage_plus::Bound;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda_cw721_offers";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 10u32;
const MAX_LIMIT: u32 = 30u32;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CW721_CONTRACT.save(deps.storage, &msg.andromeda_cw721_contract)?;
    VALID_DENOM.save(deps.storage, &msg.valid_denom)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "cw721_offers".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
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
    match msg {
        ExecuteMsg::PlaceOffer {
            token_id,
            expiration,
            offer_amount,
        } => execute_place_offer(deps, env, info, token_id, offer_amount, expiration),
        ExecuteMsg::AcceptOffer {
            token_id,
            recipient,
        } => execute_accept_offer(deps, env, info, token_id, recipient),
        ExecuteMsg::CancelOffer { token_id } => execute_cancel_offer(deps, env, info, token_id),
    }
}

fn execute_place_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    offer_amount: Uint128,
    expiration: Expiration,
) -> Result<Response, ContractError> {
    let purchaser = info.sender.as_str();
    let current_offer = offers().may_load(deps.storage, &token_id)?;
    let token_owner = get_token_owner(deps.storage, &deps.querier, token_id.clone())?;
    require(
        info.sender != token_owner,
        ContractError::TokenOwnerCannotBid {},
    )?;
    require(
        // This is to avoid situations where a user transfers the token to the purchaser thinking
        // that there is an offer up and having the purchaser pull the offer right before (not
        // necessariliy malicious, could just be a coincidence). Having a concrete time will
        // give the seller a window of guaranteed time to accept the offer.
        expiration != Expiration::Never {},
        ContractError::ExpirationMustNotBeNever {},
    )?;
    require(
        !expiration.is_expired(&env.block),
        ContractError::Expired {},
    )?;
    require(
        !query_is_archived(deps.querier, deps.storage, token_id.clone())?,
        ContractError::TokenIsArchived {},
    )?;
    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Must send one type of funds".to_string(),
        },
    )?;
    let coin: &Coin = &info.funds[0];
    let valid_denom = VALID_DENOM.load(deps.storage)?;
    require(
        valid_denom == coin.denom,
        ContractError::InvalidFunds {
            msg: "Invalid offer denom".to_string(),
        },
    )?;
    let mut msgs: Vec<SubMsg> = vec![];
    if let Some(current_offer) = current_offer {
        require(
            purchaser != current_offer.purchaser,
            ContractError::OfferAlreadyPlaced {},
        )?;
        require(
            current_offer.expiration.is_expired(&env.block)
                || current_offer.offer_amount < offer_amount,
            ContractError::OfferLowerThanCurrent {},
        )?;
        msgs.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            amount: vec![current_offer.get_full_amount()],
            to_address: current_offer.purchaser,
        })));
    }
    let res = on_funds_transfer(
        deps.storage,
        &deps.querier,
        info.sender.to_string(),
        token_id.clone(),
        Coin {
            denom: coin.denom.clone(),
            amount: offer_amount,
        },
    )?;
    let remaining_amount = res.leftover_funds.try_get_coin()?;
    let tax_amount = get_tax_amount(&res.msgs, offer_amount, remaining_amount.amount);
    let offer = Offer {
        purchaser: purchaser.to_owned(),
        denom: coin.denom.clone(),
        offer_amount,
        remaining_amount: remaining_amount.amount,
        tax_amount,
        expiration,
        msgs: res.msgs,
        events: res.events,
    };
    // require that the sender has sent enough for taxes
    require(
        has_coins(&info.funds, &offer.get_full_amount()),
        ContractError::InsufficientFunds {},
    )?;

    offers().save(deps.storage, &token_id, &offer)?;
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "place_offer")
        .add_attribute("purchaser", purchaser)
        .add_attribute("offer_amount", offer_amount)
        .add_attribute("token_id", token_id))
}

fn execute_cancel_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let offer = offers().load(deps.storage, &token_id)?;
    require(
        info.sender == offer.purchaser,
        ContractError::Unauthorized {},
    )?;
    require(
        offer.expiration.is_expired(&env.block),
        ContractError::OfferNotExpired {},
    )?;
    offers().remove(deps.storage, &token_id)?;
    let msg: SubMsg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![offer.get_full_amount()],
    }));
    Ok(Response::new()
        .add_submessage(msg)
        .add_attribute("action", "cancel_offer")
        .add_attribute("token_id", token_id))
}

fn execute_accept_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    recipient: String,
) -> Result<Response, ContractError> {
    let offer = offers().load(deps.storage, &token_id)?;
    let cw721_contract = CW721_CONTRACT.load(deps.storage)?;
    require(
        !offer.expiration.is_expired(&env.block),
        ContractError::Expired {},
    )?;
    // Only the cw721 contract can accept offers.
    require(
        info.sender == cw721_contract,
        ContractError::Unauthorized {},
    )?;
    require(
        query_transfer_agreement(deps.querier, deps.storage, token_id.clone())?.is_none(),
        ContractError::TransferAgreementExists {},
    )?;
    let payment_msg: SubMsg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient,
        amount: vec![Coin {
            amount: offer.remaining_amount,
            denom: offer.denom,
        }],
    }));

    let resp = Response::new()
        .add_submessages(offer.msgs)
        .add_submessage(payment_msg)
        .add_events(offer.events);

    offers().remove(deps.storage, &token_id)?;

    Ok(resp
        .add_attribute("action", "accept_offer")
        .add_attribute("token_id", token_id))
}

fn on_funds_transfer(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    sender: String,
    token_id: String,
    amount: Coin,
) -> Result<OnFundsTransferResponse, ContractError> {
    let address = CW721_CONTRACT.load(storage)?;
    let res: OnFundsTransferResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: address,
        msg: encode_binary(&Cw721QueryMsg::AndrHook(AndromedaHook::OnFundsTransfer {
            // Not sure sender should be this contract or the info.sender. If we get different
            // usecases in the future, using this contract as sender could allow us to have
            // separate cases for what the hook should return.
            sender,
            payload: encode_binary(&token_id)?,
            amount: Funds::Native(amount),
        }))?,
    }))?;
    Ok(res)
}

fn get_token_owner(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    token_id: String,
) -> Result<String, ContractError> {
    let address = CW721_CONTRACT.load(storage)?;
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: address,
        msg: encode_binary(&Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        })?,
    }))?;
    Ok(res.owner)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    require(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        },
    )?;

    // New version has to be newer/greater than the old version
    require(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrHook(msg) => handle_andr_hook(deps, env, msg),
        QueryMsg::Offer { token_id } => encode_binary(&query_offer(deps, token_id)?),
        QueryMsg::AllOffers {
            purchaser,
            limit,
            start_after,
        } => encode_binary(&query_all_offers(deps, purchaser, limit, start_after)?),
    }
}

fn handle_andr_hook(deps: Deps, env: Env, msg: AndromedaHook) -> Result<Binary, ContractError> {
    match msg {
        AndromedaHook::OnTransfer {
            token_id,
            sender,
            recipient,
        } => {
            let mut resp: Response = Response::new();
            let offer = offers().may_load(deps.storage, &token_id)?;
            if let Some(offer) = offer {
                if offer.purchaser == recipient {
                    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: env.contract.address.to_string(),
                        funds: vec![],
                        // The assumption is that the owner transfering the token to a user that has
                        // an offer means they want to accept that offer. If the offer is
                        // expired this message will end up failing and the transfer will not
                        // happen.
                        msg: encode_binary(&ExecuteMsg::AcceptOffer {
                            token_id,
                            // We require a recipient since the owner of the token will have
                            // changed once this message gets executed. Sender is assuemd to be the
                            // orignal owner of the token.
                            recipient: sender,
                        })?,
                    });
                    resp = resp.add_message(msg);
                }
            }

            Ok(encode_binary(&resp)?)
        }
        _ => Err(ContractError::UnsupportedOperation {}),
    }
}

fn query_offer(deps: Deps, token_id: String) -> Result<OfferResponse, ContractError> {
    Ok(offers().load(deps.storage, &token_id)?.into())
}

fn query_all_offers(
    deps: Deps,
    purchaser: String,
    limit: Option<u32>,
    start_after: Option<String>,
) -> Result<AllOffersResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let keys: Vec<String> = offers()
        .idx
        .purchaser
        .prefix(purchaser)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<Result<Vec<String>, StdError>>()?;
    let mut offer_responses: Vec<OfferResponse> = vec![];
    for key in keys.iter() {
        offer_responses.push(offers().load(deps.storage, key)?.into());
    }
    Ok(AllOffersResponse {
        offers: offer_responses,
    })
}
