use crate::state::{
    auction_infos, read_auction_infos, read_bids, BIDS, NEXT_AUCTION_ID, TOKEN_AUCTION_STATE,
};
use andromeda_non_fungible_tokens::auction::{
    AuctionIdsResponse, AuctionInfo, AuctionStateResponse, Bid, BidsResponse, Cw721HookMsg,
    ExecuteMsg, InstantiateMsg, IsCancelledResponse, IsClaimedResponse, IsClosedResponse, QueryMsg,
    TokenAuctionState,
};

use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg},
    common::Funds,
    common::{encode_binary, expiration::expiration_from_milliseconds, OrderBy},
    common::{expiration::MILLISECONDS_TO_NANOSECONDS_RATIO, rates::get_tax_amount},
    error::ContractError,
};
use andromeda_std::{
    ado_contract::ADOContract,
    common::{
        context::ExecuteContext,
        migrate::{migrate as do_migrate, MigrateMsg},
    },
};

use cosmwasm_std::{
    attr, coins, ensure, entry_point, from_json, Addr, BankMsg, Binary, BlockInfo, Coin, CosmosMsg,
    Deps, DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, Storage, SubMsg,
    Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, Cw721ReceiveMsg, Expiration, OwnerOfResponse};
use cw_utils::nonpayable;

const CONTRACT_NAME: &str = "crates.io:andromeda_auction";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    NEXT_AUCTION_ID.save(deps.storage, &Uint128::from(1u128))?;
    let contract = ADOContract::default();
    let resp = contract.instantiate(
        deps.storage,
        env,
        deps.api,
        info.clone(),
        BaseInstantiateMsg {
            ado_type: "auction".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    let modules_resp =
        contract.register_modules(info.sender.as_str(), deps.storage, msg.modules)?;

    Ok(resp
        .add_submessages(modules_resp.messages)
        .add_attributes(modules_resp.attributes))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    if !matches!(msg, ExecuteMsg::UpdateAppContract { .. })
        && !matches!(msg, ExecuteMsg::UpdateOwner { .. })
    {
        contract.module_hook::<Response>(
            &ctx.deps.as_ref(),
            AndromedaHook::OnExecute {
                sender: ctx.info.sender.to_string(),
                payload: encode_binary(&msg)?,
            },
        )?;
    }
    match msg {
        ExecuteMsg::ReceiveNft(msg) => handle_receive_cw721(ctx, msg),
        ExecuteMsg::UpdateAuction {
            token_id,
            token_address,
            start_time,
            duration,
            coin_denom,
            whitelist,
            min_bid,
        } => execute_update_auction(
            ctx,
            token_id,
            token_address,
            start_time,
            duration,
            coin_denom,
            whitelist,
            min_bid,
        ),
        ExecuteMsg::PlaceBid {
            token_id,
            token_address,
        } => execute_place_bid(ctx, token_id, token_address),
        ExecuteMsg::CancelAuction {
            token_id,
            token_address,
        } => execute_cancel(ctx, token_id, token_address),
        ExecuteMsg::Claim {
            token_id,
            token_address,
        } => execute_claim(ctx, token_id, token_address),
        _ => ADOContract::default().execute(ctx, msg),
    }
}

fn handle_receive_cw721(
    ctx: ExecuteContext,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_json(&msg.msg)? {
        Cw721HookMsg::StartAuction {
            start_time,
            duration,
            coin_denom,
            whitelist,
            min_bid,
        } => execute_start_auction(
            ctx,
            msg.sender,
            msg.token_id,
            start_time,
            duration,
            coin_denom,
            whitelist,
            min_bid,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_start_auction(
    ctx: ExecuteContext,
    sender: String,
    token_id: String,
    start_time: u64,
    duration: u64,
    coin_denom: String,
    whitelist: Option<Vec<Addr>>,
    min_bid: Option<Uint128>,
) -> Result<Response, ContractError> {
    ensure!(
        start_time > 0 && duration > 0,
        ContractError::InvalidExpiration {}
    );
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;

    let start_expiration = expiration_from_milliseconds(start_time)?;
    let end_expiration = expiration_from_milliseconds(start_time + duration)?;

    let block_time = block_to_expiration(&env.block, start_expiration).unwrap();
    ensure!(
        start_expiration.gt(&block_time),
        ContractError::StartTimeInThePast {
            current_time: env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO,
            current_block: env.block.height,
        }
    );

    let token_address = info.sender.to_string();

    let auction_id = get_and_increment_next_auction_id(deps.storage, &token_id, &token_address)?;
    BIDS.save(deps.storage, auction_id.u128(), &vec![])?;

    let whitelist_str = format!("{:?}", &whitelist);

    TOKEN_AUCTION_STATE.save(
        deps.storage,
        auction_id.u128(),
        &TokenAuctionState {
            start_time: start_expiration,
            end_time: end_expiration,
            high_bidder_addr: Addr::unchecked(""),
            high_bidder_amount: Uint128::zero(),
            coin_denom: coin_denom.clone(),
            auction_id,
            whitelist,
            min_bid,
            owner: sender,
            token_id,
            token_address,
            is_cancelled: false,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_auction"),
        attr("start_time", start_expiration.to_string()),
        attr("end_time", end_expiration.to_string()),
        attr("coin_denom", coin_denom),
        attr("auction_id", auction_id.to_string()),
        attr("whitelist", whitelist_str),
    ]))
}

#[allow(clippy::too_many_arguments)]
fn execute_update_auction(
    ctx: ExecuteContext,
    token_id: String,
    token_address: String,
    start_time: u64,
    duration: u64,
    coin_denom: String,
    whitelist: Option<Vec<Addr>>,
    min_bid: Option<Uint128>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    nonpayable(&info)?;

    let mut token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;
    ensure!(
        info.sender == token_auction_state.owner,
        ContractError::Unauthorized {}
    );
    ensure!(
        !token_auction_state.start_time.is_expired(&env.block),
        ContractError::AuctionAlreadyStarted {}
    );
    ensure!(
        start_time > 0 && duration > 0,
        ContractError::InvalidExpiration {}
    );

    let start_exp = expiration_from_milliseconds(start_time)?;
    let end_exp = expiration_from_milliseconds(start_time + duration)?;
    ensure!(
        !start_exp.is_expired(&env.block),
        ContractError::StartTimeInThePast {
            current_time: env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO,
            current_block: env.block.height,
        }
    );

    token_auction_state.start_time = start_exp;
    token_auction_state.end_time = end_exp;
    token_auction_state.whitelist = whitelist.clone();
    token_auction_state.coin_denom = coin_denom.clone();
    token_auction_state.min_bid = min_bid;
    TOKEN_AUCTION_STATE.save(
        deps.storage,
        token_auction_state.auction_id.u128(),
        &token_auction_state,
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "update_auction"),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_exp.to_string()),
        attr("coin_denom", coin_denom),
        attr("auction_id", token_auction_state.auction_id.to_string()),
        attr("whitelist", format!("{:?}", &whitelist)),
        attr("min_bid", format!("{:?}", &min_bid)),
    ]))
}

fn execute_place_bid(
    ctx: ExecuteContext,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    let mut token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;

    ensure!(
        !token_auction_state.is_cancelled,
        ContractError::AuctionCancelled {}
    );

    ensure!(
        token_auction_state.start_time.is_expired(&env.block),
        ContractError::AuctionNotStarted {}
    );
    ensure!(
        !token_auction_state.end_time.is_expired(&env.block),
        ContractError::AuctionEnded {}
    );

    ensure!(
        token_auction_state.owner != info.sender,
        ContractError::TokenOwnerCannotBid {}
    );

    ensure!(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Auctions ensure! exactly one coin to be sent.".to_string(),
        }
    );
    if let Some(ref whitelist) = token_auction_state.whitelist {
        ensure!(
            whitelist.iter().any(|x| x == info.sender),
            ContractError::Unauthorized {}
        );
    }

    ensure!(
        token_auction_state.high_bidder_addr != info.sender,
        ContractError::HighestBidderCannotOutBid {}
    );

    let payment: &Coin = &info.funds[0];
    ensure!(
        payment.denom == token_auction_state.coin_denom && payment.amount > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: format!(
                "No {} assets are provided to auction",
                token_auction_state.coin_denom
            ),
        }
    );
    let min_bid = token_auction_state.min_bid.unwrap_or(Uint128::zero());
    ensure!(
        payment.amount >= min_bid,
        ContractError::InvalidFunds {
            msg: format!(
                "Must provide at least {min_bid} {} to bid",
                token_auction_state.coin_denom
            )
        }
    );
    ensure!(
        token_auction_state.high_bidder_amount < payment.amount,
        ContractError::BidSmallerThanHighestBid {}
    );

    let mut messages: Vec<CosmosMsg> = vec![];
    // Send back previous bid unless there was no previous bid.
    if token_auction_state.high_bidder_amount > Uint128::zero() {
        let bank_msg = BankMsg::Send {
            to_address: token_auction_state.high_bidder_addr.to_string(),
            amount: coins(
                token_auction_state.high_bidder_amount.u128(),
                token_auction_state.coin_denom.clone(),
            ),
        };
        messages.push(CosmosMsg::Bank(bank_msg));
    }

    token_auction_state.high_bidder_addr = info.sender.clone();
    token_auction_state.high_bidder_amount = payment.amount;

    let key = token_auction_state.auction_id.u128();
    TOKEN_AUCTION_STATE.save(deps.storage, key, &token_auction_state)?;
    let mut bids_for_auction = BIDS.load(deps.storage, key)?;
    bids_for_auction.push(Bid {
        bidder: info.sender.to_string(),
        amount: payment.amount,
        timestamp: env.block.time,
    });
    BIDS.save(deps.storage, key, &bids_for_auction)?;
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "bid"),
        attr("token_id", token_id),
        attr("bider", info.sender.to_string()),
        attr("amount", payment.amount.to_string()),
    ]))
}

fn execute_cancel(
    ctx: ExecuteContext,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    nonpayable(&info)?;

    let mut token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;
    ensure!(
        info.sender == token_auction_state.owner,
        ContractError::Unauthorized {}
    );
    ensure!(
        !token_auction_state.end_time.is_expired(&env.block),
        ContractError::AuctionEnded {}
    );
    let mut messages: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_auction_state.token_address.clone(),
        msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: info.sender.to_string(),
            token_id,
        })?,
        funds: vec![],
    })];

    // Refund highest bid, if it exists.
    if !token_auction_state.high_bidder_amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: token_auction_state.high_bidder_addr.to_string(),
            amount: coins(
                token_auction_state.high_bidder_amount.u128(),
                token_auction_state.coin_denom.clone(),
            ),
        }));
    }

    token_auction_state.is_cancelled = true;
    TOKEN_AUCTION_STATE.save(
        deps.storage,
        token_auction_state.auction_id.u128(),
        &token_auction_state,
    )?;

    Ok(Response::new().add_messages(messages))
}

fn execute_claim(
    ctx: ExecuteContext,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    nonpayable(&info)?;
    let token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;
    ensure!(
        token_auction_state.end_time.is_expired(&env.block),
        ContractError::AuctionNotEnded {}
    );
    let token_owner = query_owner_of(
        deps.querier,
        token_auction_state.token_address.clone(),
        token_id.clone(),
    )?
    .owner;
    ensure!(
        // If this is false then the token is no longer held by the contract so the token has been
        // claimed.
        token_owner == env.contract.address,
        ContractError::AuctionAlreadyClaimed {}
    );
    // This is the case where no-one bid on the token.
    if token_auction_state.high_bidder_addr.to_string().is_empty()
        || token_auction_state.high_bidder_amount.is_zero()
    {
        return Ok(Response::new()
            // Send NFT back to the original owner.
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_auction_state.token_address.clone(),
                msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: token_auction_state.owner.clone(),
                    token_id: token_id.clone(),
                })?,
                funds: vec![],
            }))
            .add_attribute("action", "claim")
            .add_attribute("token_id", token_id)
            .add_attribute("token_contract", token_auction_state.token_address)
            .add_attribute("recipient", token_auction_state.owner)
            .add_attribute("winning_bid_amount", token_auction_state.high_bidder_amount)
            .add_attribute("auction_id", token_auction_state.auction_id));
    }

    // Calculate the funds to be received after tax
    let after_tax_payment = purchase_token(deps.as_ref(), &info, token_auction_state.clone())?;

    Ok(Response::new()
        .add_submessages(after_tax_payment.1)
        // Send funds to the original owner.
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: token_auction_state.owner,
            amount: vec![after_tax_payment.0],
        }))
        // Send NFT to auction winner.
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_auction_state.token_address.clone(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: token_auction_state.high_bidder_addr.to_string(),
                token_id: token_id.clone(),
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "claim")
        .add_attribute("token_id", token_id)
        .add_attribute("token_contract", token_auction_state.token_address)
        .add_attribute("recipient", &token_auction_state.high_bidder_addr)
        .add_attribute("winning_bid_amount", token_auction_state.high_bidder_amount)
        .add_attribute("auction_id", token_auction_state.auction_id))
}

fn purchase_token(
    deps: Deps,
    info: &MessageInfo,
    state: TokenAuctionState,
) -> Result<(Coin, Vec<SubMsg>), ContractError> {
    let total_cost = Coin::new(state.high_bidder_amount.u128(), state.coin_denom.clone());

    let mut total_tax_amount = Uint128::zero();

    let (msgs, _events, remainder) = ADOContract::default().on_funds_transfer(
        &deps,
        info.sender.to_string(),
        Funds::Native(total_cost),
        encode_binary(&"")?,
    )?;

    let remaining_amount = remainder.try_get_coin()?;

    let tax_amount = get_tax_amount(&msgs, state.high_bidder_amount, remaining_amount.amount);

    // Calculate total tax
    total_tax_amount += tax_amount;

    let after_tax_payment = Coin {
        denom: state.coin_denom,
        amount: remaining_amount.amount,
    };
    Ok((after_tax_payment, msgs))
}

fn get_existing_token_auction_state(
    storage: &dyn Storage,
    token_id: &str,
    token_address: &str,
) -> Result<TokenAuctionState, ContractError> {
    let key = token_id.to_owned() + token_address;
    let latest_auction_id: Uint128 = match auction_infos().may_load(storage, &key)? {
        None => return Err(ContractError::AuctionDoesNotExist {}),
        Some(auction_info) => *auction_info.last().unwrap(),
    };
    let token_auction_state = TOKEN_AUCTION_STATE.load(storage, latest_auction_id.u128())?;

    Ok(token_auction_state)
}

fn block_to_expiration(block: &BlockInfo, model: Expiration) -> Option<Expiration> {
    match model {
        Expiration::AtTime(_) => Some(Expiration::AtTime(block.time)),
        Expiration::AtHeight(_) => Some(Expiration::AtHeight(block.height)),
        Expiration::Never {} => None,
    }
}

fn get_and_increment_next_auction_id(
    storage: &mut dyn Storage,
    token_id: &str,
    token_address: &str,
) -> Result<Uint128, ContractError> {
    let next_auction_id = NEXT_AUCTION_ID.load(storage)?;
    let incremented_next_auction_id = next_auction_id.checked_add(Uint128::from(1u128))?;
    NEXT_AUCTION_ID.save(storage, &incremented_next_auction_id)?;

    let key = token_id.to_owned() + token_address;

    let mut auction_info = auction_infos().load(storage, &key).unwrap_or_default();
    auction_info.push(next_auction_id);
    if auction_info.token_address.is_empty() {
        auction_info.token_address = token_address.to_owned();
        auction_info.token_id = token_id.to_owned();
    }
    auction_infos().save(storage, &key, &auction_info)?;
    Ok(next_auction_id)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::LatestAuctionState {
            token_id,
            token_address,
        } => encode_binary(&query_latest_auction_state(deps, token_id, token_address)?),
        QueryMsg::AuctionState { auction_id } => {
            encode_binary(&query_auction_state(deps, auction_id)?)
        }
        QueryMsg::Bids {
            auction_id,
            start_after,
            limit,
            order_by,
        } => encode_binary(&query_bids(deps, auction_id, start_after, limit, order_by)?),
        QueryMsg::AuctionIds {
            token_id,
            token_address,
        } => encode_binary(&query_auction_ids(deps, token_id, token_address)?),
        QueryMsg::AuctionInfosForAddress {
            token_address,
            start_after,
            limit,
        } => encode_binary(&query_auction_infos_for_address(
            deps,
            token_address,
            start_after,
            limit,
        )?),
        QueryMsg::IsCancelled {
            token_id,
            token_address,
        } => encode_binary(&query_is_cancelled(deps, token_id, token_address)?),
        QueryMsg::IsClaimed {
            token_id,
            token_address,
        } => encode_binary(&query_is_claimed(deps, env, token_id, token_address)?),
        QueryMsg::IsClosed {
            token_id,
            token_address,
        } => encode_binary(&query_is_closed(deps, env, token_id, token_address)?),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_is_cancelled(
    deps: Deps,
    token_id: String,
    token_address: String,
) -> Result<IsCancelledResponse, ContractError> {
    let token_auction_state_result =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address);
    if let Ok(token_auction_state) = token_auction_state_result {
        return Ok(IsCancelledResponse {
            is_cancelled: token_auction_state.is_cancelled,
        });
    }
    Err(ContractError::AuctionDoesNotExist {})
}

fn query_is_claimed(
    deps: Deps,
    env: Env,
    token_id: String,
    token_address: String,
) -> Result<IsClaimedResponse, ContractError> {
    let token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;

    let token_owner =
        query_owner_of(deps.querier, token_auction_state.token_address, token_id)?.owner;

    // if token owner isn't the contract, it means that it has been claimed. If they're equal it means that it hasn't been claimed and will return false
    Ok(IsClaimedResponse {
        is_claimed: token_owner != env.contract.address,
    })
}

fn query_is_closed(
    deps: Deps,
    env: Env,
    token_id: String,
    token_address: String,
) -> Result<IsClosedResponse, ContractError> {
    let token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;

    let is_claimed =
        query_is_claimed(deps, env.clone(), token_id.clone(), token_address.clone())?.is_claimed;
    let is_cancelled = query_is_cancelled(deps, token_id, token_address)?.is_cancelled;
    let is_expired = token_auction_state.end_time.is_expired(&env.block);

    // Considers the auction closed if one or more of those 3 variables are true. Otherwise it isn't closed.
    let is_closed = is_claimed || is_cancelled || is_expired;
    Ok(IsClosedResponse { is_closed })
}

fn query_auction_ids(
    deps: Deps,
    token_id: String,
    token_address: String,
) -> Result<AuctionIdsResponse, ContractError> {
    let key = token_id + &token_address;
    let auction_info = auction_infos().may_load(deps.storage, &key)?;
    if let Some(auction_info) = auction_info {
        return Ok(AuctionIdsResponse {
            auction_ids: auction_info.auction_ids,
        });
    }
    Ok(AuctionIdsResponse {
        auction_ids: vec![],
    })
}

pub fn query_auction_infos_for_address(
    deps: Deps,
    token_address: String,
    start_after: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<AuctionInfo>, ContractError> {
    read_auction_infos(deps.storage, token_address, start_after, limit)
}

fn query_bids(
    deps: Deps,
    auction_id: Uint128,
    start_after: Option<u64>,
    limit: Option<u64>,
    order_by: Option<OrderBy>,
) -> Result<BidsResponse, ContractError> {
    let bids = read_bids(
        deps.storage,
        auction_id.u128(),
        start_after,
        limit,
        order_by,
    )?;
    Ok(BidsResponse { bids })
}

fn query_latest_auction_state(
    deps: Deps,
    token_id: String,
    token_address: String,
) -> Result<AuctionStateResponse, ContractError> {
    let token_auction_state_result =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address);
    if let Ok(token_auction_state) = token_auction_state_result {
        return Ok(token_auction_state.into());
    }
    Err(ContractError::AuctionDoesNotExist {})
}

fn query_auction_state(
    deps: Deps,
    auction_id: Uint128,
) -> Result<AuctionStateResponse, ContractError> {
    let token_auction_state = TOKEN_AUCTION_STATE.load(deps.storage, auction_id.u128())?;
    Ok(token_auction_state.into())
}

fn query_owner_of(
    querier: QuerierWrapper,
    token_addr: String,
    token_id: String,
) -> Result<OwnerOfResponse, ContractError> {
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_addr,
        msg: encode_binary(&Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        })?,
    }))?;

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    do_migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
