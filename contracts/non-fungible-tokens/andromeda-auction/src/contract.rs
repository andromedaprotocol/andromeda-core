use crate::state::{
    auction_infos, read_auction_infos, read_bids, AuctionInfo, TokenAuctionState, BIDS,
    NEXT_AUCTION_ID, TOKEN_AUCTION_STATE,
};
use ado_base::state::ADOContract;
use andromeda_non_fungible_tokens::auction::{
    AuctionIdsResponse, AuctionStateResponse, Bid, BidsResponse, Cw721HookMsg, ExecuteMsg,
    InstantiateMsg, MigrateMsg, QueryMsg,
};
use common::{
    ado_base::{InstantiateMsg as BaseInstantiateMsg, hooks::AndromedaHook}, encode_binary, error::ContractError,
    rates::get_tax_amount, require, Funds, OrderBy,
};
use cosmwasm_std::{
    attr, coins, entry_point, from_binary, Addr, Api, BankMsg, Binary, BlockInfo, Coin, CosmosMsg,
    Deps, DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, Storage,
    SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, Cw721ReceiveMsg, Expiration, OwnerOfResponse};
use cw_utils::nonpayable;
use semver::Version;

const CONTRACT_NAME: &str = "crates.io:andromeda_auction";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    NEXT_AUCTION_ID.save(deps.storage, &Uint128::from(1u128))?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        BaseInstantiateMsg {
            ado_type: "auction".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            modules: msg.modules,
            primitive_contract: None,
        },
    )
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = ADOContract::default();

    contract.module_hook::<Response>(
        deps.storage,
        deps.api,
        deps.querier,
        AndromedaHook::OnExecute {
            sender: info.sender.to_string(),
            payload: encode_binary(&msg)?,
        },
    )?;
    
    match msg {
        ExecuteMsg::AndrReceive(msg) => {
            ADOContract::default().execute(deps, env, info, msg, execute)
        }
        ExecuteMsg::ReceiveNft(msg) => handle_receive_cw721(deps, env, info, msg),
        ExecuteMsg::UpdateAuction {
            token_id,
            token_address,
            start_time,
            end_time,
            coin_denom,
            whitelist,
        } => execute_update_auction(
            deps,
            env,
            info,
            token_id,
            token_address,
            start_time,
            end_time,
            coin_denom,
            whitelist,
        ),
        ExecuteMsg::PlaceBid {
            token_id,
            token_address,
        } => execute_place_bid(deps, env, info, token_id, token_address),
        ExecuteMsg::CancelAuction {
            token_id,
            token_address,
        } => execute_cancel(deps, env, info, token_id, token_address),
        ExecuteMsg::Claim {
            token_id,
            token_address,
        } => execute_claim(deps, env, info, token_id, token_address),
        ExecuteMsg::UpdateOwner { address } => {
            ADOContract::default().execute_update_owner(deps, info, address)
        }
    }
}

fn handle_receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&msg.msg)? {
        Cw721HookMsg::StartAuction {
            start_time,
            end_time,
            coin_denom,
            whitelist,
        } => execute_start_auction(
            deps,
            env,
            msg.sender,
            msg.token_id,
            info.sender.to_string(),
            start_time,
            end_time,
            coin_denom,
            whitelist,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_start_auction(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_id: String,
    token_address: String,
    start_time: Expiration,
    end_time: Expiration,
    coin_denom: String,
    whitelist: Option<Vec<Addr>>,
) -> Result<Response, ContractError> {
    require(
        start_time != Expiration::Never {} && end_time != Expiration::Never {},
        ContractError::ExpirationMustNotBeNever {},
    )?;
    require(
        start_time.partial_cmp(&end_time) != None,
        ContractError::ExpirationsMustBeOfSameType {},
    )?;
    require(
        start_time < end_time,
        ContractError::StartTimeAfterEndTime {},
    )?;
    let block_time = block_to_expiration(&env.block, start_time).unwrap();
    require(
        start_time > block_time,
        ContractError::StartTimeInThePast {
            current_seconds: env.block.time.seconds(),
            current_block: env.block.height,
        },
    )?;

    let auction_id = get_and_increment_next_auction_id(deps.storage, &token_id, &token_address)?;
    BIDS.save(deps.storage, auction_id.u128(), &vec![])?;

    let whitelist_str = format!("{:?}", &whitelist);

    TOKEN_AUCTION_STATE.save(
        deps.storage,
        auction_id.u128(),
        &TokenAuctionState {
            start_time,
            end_time,
            high_bidder_addr: Addr::unchecked(""),
            high_bidder_amount: Uint128::zero(),
            coin_denom: coin_denom.clone(),
            auction_id,
            whitelist,
            owner: sender,
            token_id,
            token_address,
            is_cancelled: false,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_auction"),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_time.to_string()),
        attr("coin_denom", coin_denom),
        attr("auction_id", auction_id.to_string()),
        attr("whitelist", whitelist_str),
    ]))
}

#[allow(clippy::too_many_arguments)]
fn execute_update_auction(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
    start_time: Expiration,
    end_time: Expiration,
    coin_denom: String,
    whitelist: Option<Vec<Addr>>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let mut token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;
    require(
        info.sender == token_auction_state.owner,
        ContractError::Unauthorized {},
    )?;
    require(
        !token_auction_state.start_time.is_expired(&env.block),
        ContractError::AuctionAlreadyStarted {},
    )?;
    require(
        start_time != Expiration::Never {} && end_time != Expiration::Never {},
        ContractError::ExpirationMustNotBeNever {},
    )?;
    require(
        start_time.partial_cmp(&end_time) != None,
        ContractError::ExpirationsMustBeOfSameType {},
    )?;
    require(
        start_time < end_time,
        ContractError::StartTimeAfterEndTime {},
    )?;
    require(
        !start_time.is_expired(&env.block),
        ContractError::StartTimeInThePast {
            current_seconds: env.block.time.seconds(),
            current_block: env.block.height,
        },
    )?;

    token_auction_state.start_time = start_time;
    token_auction_state.end_time = end_time;
    token_auction_state.whitelist = whitelist.clone();
    token_auction_state.coin_denom = coin_denom.clone();
    TOKEN_AUCTION_STATE.save(
        deps.storage,
        token_auction_state.auction_id.u128(),
        &token_auction_state,
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_auction"),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_time.to_string()),
        attr("coin_denom", coin_denom),
        attr("auction_id", token_auction_state.auction_id.to_string()),
        attr("whitelist", format!("{:?}", &whitelist)),
    ]))
}

fn execute_place_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let mut token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;

    require(
        !token_auction_state.is_cancelled,
        ContractError::AuctionCancelled {},
    )?;

    require(
        token_auction_state.start_time.is_expired(&env.block),
        ContractError::AuctionNotStarted {},
    )?;
    require(
        !token_auction_state.end_time.is_expired(&env.block),
        ContractError::AuctionEnded {},
    )?;

    require(
        token_auction_state.owner != info.sender,
        ContractError::TokenOwnerCannotBid {},
    )?;

    require(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Auctions require exactly one coin to be sent.".to_string(),
        },
    )?;
    if let Some(ref whitelist) = token_auction_state.whitelist {
        require(
            whitelist.iter().any(|x| x == &info.sender),
            ContractError::Unauthorized {},
        )?;
    }

    require(
        token_auction_state.high_bidder_addr != info.sender,
        ContractError::HighestBidderCannotOutBid {},
    )?;

    let coin_denom = token_auction_state.coin_denom.clone();
    let payment: &Coin = &info.funds[0];
    require(
        payment.denom == coin_denom && payment.amount > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: format!("No {} assets are provided to auction", coin_denom),
        },
    )?;
    require(
        token_auction_state.high_bidder_amount < payment.amount,
        ContractError::BidSmallerThanHighestBid {},
    )?;

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
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let mut token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;
    require(
        info.sender == token_auction_state.owner,
        ContractError::Unauthorized {},
    )?;
    require(
        !token_auction_state.end_time.is_expired(&env.block),
        ContractError::AuctionEnded {},
    )?;
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
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let token_auction_state =
        get_existing_token_auction_state(deps.storage, &token_id, &token_address)?;
    require(
        token_auction_state.end_time.is_expired(&env.block),
        ContractError::AuctionNotEnded {},
    )?;
    let token_owner = query_owner_of(
        deps.querier,
        token_auction_state.token_address.clone(),
        token_id.clone(),
    )?
    .owner;
    require(
        // If this is false then the token is no longer held by the contract so the token has been
        // claimed.
        token_owner == env.contract.address,
        ContractError::AuctionAlreadyClaimed {},
    )?;
    // This is the case where no-one bid on the token.
    if token_auction_state.high_bidder_amount.is_zero() {
        return Ok(Response::new()
            // Send NFT back to the original owner.
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_auction_state.token_address.clone(),
                msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: token_auction_state.owner,
                    token_id: token_id.clone(),
                })?,
                funds: vec![],
            }))
            .add_attribute("action", "claim")
            .add_attribute("token_id", token_id)
            .add_attribute("token_contract", token_auction_state.token_address)
            .add_attribute("recipient", &token_auction_state.high_bidder_addr)
            .add_attribute("winning_bid_amount", token_auction_state.high_bidder_amount)
            .add_attribute("auction_id", token_auction_state.auction_id));
    }

    // Calculate the funds to be received after tax
    let after_tax_payment = purchase_token(
        deps.storage,
        deps.api,
        &deps.querier,
        &info,
        token_auction_state.clone(),
    )?;

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
    storage: &mut dyn Storage,
    api: &dyn Api,
    querier: &QuerierWrapper,
    info: &MessageInfo,
    state: TokenAuctionState,
) -> Result<(Coin, Vec<SubMsg>), ContractError> {
    let total_cost = Coin::new(state.high_bidder_amount.u128(), state.coin_denom.clone());

    let mut total_tax_amount = Uint128::zero();

    let (msgs, events, remainder) = ADOContract::default().on_funds_transfer(
        storage,
        api,
        querier,
        info.sender.to_string(),
        Funds::Native(total_cost),
        encode_binary(&"")?,
    )?;

    let remaining_amount = remainder.try_get_coin()?;

    let tax_amount = get_tax_amount(&msgs, state.high_bidder_amount, remaining_amount.amount);

    // Calculate total tax
    total_tax_amount += tax_amount;

    if events.iter().any(|x| x.ty == "tax") {
        let after_tax_payment = Coin {
            denom: state.coin_denom,
            amount: state.high_bidder_amount - tax_amount,
        };
        Ok((after_tax_payment, msgs))
    } else {
        let after_tax_payment = Coin {
            denom: state.coin_denom,
            amount: remaining_amount.amount,
        };
        Ok((after_tax_payment, msgs))
    }
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

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::AndrQuery(msg) => ADOContract::default().query(deps, env, msg, query),
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
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{
        mock_dependencies_custom, MOCK_TOKEN_ADDR, MOCK_TOKEN_OWNER, MOCK_UNCLAIMED_TOKEN,
    };
    use crate::state::AuctionInfo;
    use andromeda_non_fungible_tokens::auction::{Cw721HookMsg, ExecuteMsg, InstantiateMsg};
    use andromeda_testing::testing::mock_querier::{MOCK_RATES_CONTRACT, MOCK_RATES_RECIPIENT};
    use common::ado_base::modules::Module;
    use common::app::AndrAddress;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coin, coins, from_binary, BankMsg, CosmosMsg, Response, Timestamp};
    use cw721::Expiration;

    fn query_latest_auction_state_helper(deps: Deps, env: Env) -> AuctionStateResponse {
        let query_msg = QueryMsg::LatestAuctionState {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_owned(),
        };
        from_binary(&query(deps, env, query_msg).unwrap()).unwrap()
    }

    fn start_auction(deps: DepsMut, whitelist: Option<Vec<Addr>>) {
        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            coin_denom: "uusd".to_string(),
            whitelist,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let _res = execute(deps, env, info, msg).unwrap();
    }

    fn assert_auction_created(deps: Deps, whitelist: Option<Vec<Addr>>) {
        assert_eq!(
            TokenAuctionState {
                start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
                end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
                high_bidder_addr: Addr::unchecked(""),
                high_bidder_amount: Uint128::zero(),
                coin_denom: "uusd".to_string(),
                auction_id: 1u128.into(),
                whitelist,
                owner: MOCK_TOKEN_OWNER.to_string(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
                token_address: MOCK_TOKEN_ADDR.to_owned(),
                is_cancelled: false,
            },
            TOKEN_AUCTION_STATE.load(deps.storage, 1u128).unwrap()
        );

        assert_eq!(
            AuctionInfo {
                auction_ids: vec![Uint128::from(1u128)],
                token_address: MOCK_TOKEN_ADDR.to_owned(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            },
            auction_infos()
                .load(
                    deps.storage,
                    &(MOCK_UNCLAIMED_TOKEN.to_owned() + MOCK_TOKEN_ADDR)
                )
                .unwrap()
        );
    }

    #[test]
    fn test_auction_instantiate() {
        let owner = "creator";
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg { modules: None };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_place_bid_non_existing_auction() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };
        let info = mock_info("bidder", &coins(100, "uusd"));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_auction_not_started() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        assert_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(50u64);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionNotStarted {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_auction_ended() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        assert_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(300);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionEnded {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_token_owner_cannot_bid() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        assert_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info(MOCK_TOKEN_OWNER, &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::TokenOwnerCannotBid {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_whitelist() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), Some(vec![Addr::unchecked("sender")]));
        assert_auction_created(deps.as_ref(), Some(vec![Addr::unchecked("sender")]));

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let info = mock_info("not_sender", &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    }

    #[test]
    fn execute_place_bid_highest_bidder_cannot_outbid() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        assert_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("sender", &coins(200, "uusd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(
            ContractError::HighestBidderCannotOutBid {},
            res.unwrap_err()
        );
    }

    #[test]
    fn execute_place_bid_bid_smaller_than_highest_bid() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        assert_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("other", &coins(50, "uusd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::BidSmallerThanHighestBid {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_invalid_coins_sent() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        assert_auction_created(deps.as_ref(), None);

        env.block.time = Timestamp::from_seconds(150);

        let error = ContractError::InvalidFunds {
            msg: "Auctions require exactly one coin to be sent.".to_string(),
        };
        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        // No coins sent
        let info = mock_info("sender", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(error, res.unwrap_err());

        // Multiple coins sent
        let info = mock_info("sender", &[coin(100, "uusd"), coin(100, "uluna")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(error, res.unwrap_err());

        let error = ContractError::InvalidFunds {
            msg: "No uusd assets are provided to auction".to_string(),
        };

        // Invalid denom sent
        let info = mock_info("sender", &[coin(100, "uluna")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(error, res.unwrap_err());

        // Correct denom but empty
        let info = mock_info("sender", &[coin(0, "uusd")]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(error, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_multiple_bids() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        assert_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            Response::new().add_attributes(vec![
                attr("action", "bid"),
                attr("token_id", MOCK_UNCLAIMED_TOKEN),
                attr("bider", info.sender),
                attr("amount", "100"),
            ]),
            res
        );
        let mut expected_response = AuctionStateResponse {
            start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            high_bidder_addr: "sender".to_string(),
            high_bidder_amount: Uint128::from(100u128),
            auction_id: Uint128::from(1u128),
            coin_denom: "uusd".to_string(),
            whitelist: None,
            is_cancelled: false,
        };

        let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        assert_eq!(expected_response, res);

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("other", &coins(200, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "sender".to_string(),
                    amount: coins(100, "uusd")
                }))
                .add_attributes(vec![
                    attr("action", "bid"),
                    attr("token_id", MOCK_UNCLAIMED_TOKEN),
                    attr("bider", info.sender),
                    attr("amount", "200"),
                ]),
            res
        );

        expected_response.high_bidder_addr = "other".to_string();
        expected_response.high_bidder_amount = Uint128::from(200u128);
        let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        assert_eq!(expected_response, res);

        env.block.time = Timestamp::from_seconds(170);
        let info = mock_info("sender", &coins(250, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "other".to_string(),
                    amount: coins(200, "uusd")
                }))
                .add_attributes(vec![
                    attr("action", "bid"),
                    attr("token_id", MOCK_UNCLAIMED_TOKEN),
                    attr("bider", info.sender),
                    attr("amount", "250"),
                ]),
            res
        );

        expected_response.high_bidder_addr = "sender".to_string();
        expected_response.high_bidder_amount = Uint128::from(250u128);
        let res = query_latest_auction_state_helper(deps.as_ref(), env);
        assert_eq!(expected_response, res);
    }

    #[test]
    fn execute_place_bid_auction_cancelled() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        assert_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::CancelAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionCancelled {}, res.unwrap_err());
    }

    #[test]
    fn execute_start_auction() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "start_auction"),
                attr("start_time", "expiration time: 100.000000000"),
                attr("end_time", "expiration time: 200.000000000"),
                attr("coin_denom", "uusd"),
                attr("auction_id", "1"),
                attr("whitelist", "None"),
            ]),
        );
        assert_auction_created(deps.as_ref(), None);
    }

    #[test]
    fn execute_start_auction_with_block_height() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::AtHeight(100),
            end_time: Expiration::AtHeight(200),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.height = 0;

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "start_auction"),
                attr("start_time", "expiration height: 100"),
                attr("end_time", "expiration height: 200"),
                attr("coin_denom", "uusd"),
                attr("auction_id", "1"),
                attr("whitelist", "None"),
            ]),
        );
    }

    #[test]
    fn execute_start_auction_with_mismatched_expirations() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::AtHeight(100),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.height = 0;

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(
            ContractError::ExpirationsMustBeOfSameType {},
            res.unwrap_err()
        );
    }

    #[test]
    fn execute_start_auction_start_time_in_past() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg);

        assert_eq!(
            ContractError::StartTimeInThePast {
                current_seconds: env.block.time.seconds(),
                current_block: env.block.height,
            },
            res.unwrap_err()
        );
    }

    #[test]
    fn execute_start_auction_start_time_after_end_time() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(100)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(ContractError::StartTimeAfterEndTime {}, res.unwrap_err());
    }

    #[test]
    fn execute_start_auction_start_time_never() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::Never {},
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(ContractError::ExpirationMustNotBeNever {}, res.unwrap_err());
    }

    #[test]
    fn execute_start_auction_end_time_never() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartAuction {
            end_time: Expiration::Never {},
            start_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(ContractError::ExpirationMustNotBeNever {}, res.unwrap_err());
    }

    #[test]
    fn execute_update_auction_with_mismatched_expirations() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::UpdateAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            start_time: Expiration::AtHeight(100),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let mut env = mock_env();
        env.block.height = 0;
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(
            ContractError::ExpirationsMustBeOfSameType {},
            res.unwrap_err()
        );
    }

    #[test]
    fn execute_update_auction_start_time_in_past() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::UpdateAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            start_time: Expiration::AtHeight(100),
            end_time: Expiration::AtHeight(200),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let mut env = mock_env();
        env.block.height = 150;
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg);

        assert_eq!(
            ContractError::StartTimeInThePast {
                current_seconds: env.block.time.seconds(),
                current_block: env.block.height,
            },
            res.unwrap_err()
        );
    }

    #[test]
    fn execute_update_auction_start_time_after_end_time() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::UpdateAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            start_time: Expiration::AtHeight(200),
            end_time: Expiration::AtHeight(100),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let mut env = mock_env();
        env.block.height = 0;
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(ContractError::StartTimeAfterEndTime {}, res.unwrap_err());
    }

    #[test]
    fn execute_update_auction_start_time_never() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::UpdateAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            start_time: Expiration::Never {},
            end_time: Expiration::AtHeight(200),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(ContractError::ExpirationMustNotBeNever {}, res.unwrap_err());
    }

    #[test]
    fn execute_update_auction_end_time_never() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::UpdateAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            start_time: Expiration::AtHeight(100),
            end_time: Expiration::Never {},
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(ContractError::ExpirationMustNotBeNever {}, res.unwrap_err());
    }

    #[test]
    fn execute_update_auction_unauthorized() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::UpdateAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            start_time: Expiration::AtHeight(100),
            end_time: Expiration::AtHeight(200),
            coin_denom: "uluna".to_string(),
            whitelist: Some(vec![Addr::unchecked("user")]),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(150);
        env.block.height = 0;

        let info = mock_info("not_owner", &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn execute_update_auction_auction_started() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::UpdateAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            start_time: Expiration::AtHeight(100),
            end_time: Expiration::AtHeight(200),
            coin_denom: "uluna".to_string(),
            whitelist: Some(vec![Addr::unchecked("user")]),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(150);
        env.block.height = 0;

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionAlreadyStarted {}, res.unwrap_err());
    }

    #[test]
    fn execute_update_auction() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::UpdateAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
            start_time: Expiration::AtHeight(100),
            end_time: Expiration::AtHeight(200),
            coin_denom: "uluna".to_string(),
            whitelist: Some(vec![Addr::unchecked("user")]),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        env.block.height = 0;

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            TokenAuctionState {
                start_time: Expiration::AtHeight(100),
                end_time: Expiration::AtHeight(200),
                high_bidder_addr: Addr::unchecked(""),
                high_bidder_amount: Uint128::zero(),
                coin_denom: "uluna".to_string(),
                auction_id: 1u128.into(),
                whitelist: Some(vec![Addr::unchecked("user")]),
                owner: MOCK_TOKEN_OWNER.to_string(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
                token_address: MOCK_TOKEN_ADDR.to_owned(),
                is_cancelled: false,
            },
            TOKEN_AUCTION_STATE
                .load(deps.as_ref().storage, 1u128)
                .unwrap()
        );
    }

    #[test]
    fn execute_start_auction_after_previous_finished() {
        let mut deps = mock_dependencies_custom(&[]);
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // There was a previous auction.
        start_auction(deps.as_mut(), None);

        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::AtTime(Timestamp::from_seconds(300)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(400)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(250);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            Response::new().add_attributes(vec![
                attr("action", "start_auction"),
                attr("start_time", "expiration time: 300.000000000"),
                attr("end_time", "expiration time: 400.000000000"),
                attr("coin_denom", "uusd"),
                attr("auction_id", "2"),
                attr("whitelist", "None"),
            ]),
            res
        );
    }

    #[test]
    fn execute_claim_no_bids() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        env.block.time = Timestamp::from_seconds(250);

        let msg = ExecuteMsg::Claim {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_TOKEN_ADDR.to_owned(),
                    msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                        recipient: MOCK_TOKEN_OWNER.to_owned(),
                        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
                    })
                    .unwrap(),
                    funds: vec![],
                }))
                .add_attribute("action", "claim")
                .add_attribute("token_id", MOCK_UNCLAIMED_TOKEN)
                .add_attribute("token_contract", MOCK_TOKEN_ADDR)
                .add_attribute("recipient", "")
                .add_attribute("winning_bid_amount", Uint128::zero())
                .add_attribute("auction_id", "1"),
            res
        );
    }

    #[test]
    fn execute_claim() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        env.block.time = Timestamp::from_seconds(250);

        let msg = ExecuteMsg::Claim {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let transfer_nft_msg = Cw721ExecuteMsg::TransferNft {
            recipient: "sender".to_string(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        };
        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: MOCK_TOKEN_OWNER.to_owned(),
                    amount: coins(100, "uusd"),
                }))
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_TOKEN_ADDR.to_string(),
                    msg: encode_binary(&transfer_nft_msg).unwrap(),
                    funds: vec![],
                }))
                .add_attribute("action", "claim")
                .add_attribute("token_id", MOCK_UNCLAIMED_TOKEN)
                .add_attribute("token_contract", MOCK_TOKEN_ADDR)
                .add_attribute("recipient", "sender")
                .add_attribute("winning_bid_amount", Uint128::from(100u128))
                .add_attribute("auction_id", "1"),
            res
        );
    }

    #[test]
    fn execute_claim_with_modules() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let module = Module {
            module_type: "rates".to_string(),
            address: AndrAddress {
                identifier: MOCK_RATES_CONTRACT.to_owned(),
            },
            is_mutable: true,
        };
        let msg = InstantiateMsg {
            modules: Some(vec![module]),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        env.block.time = Timestamp::from_seconds(250);

        let msg = ExecuteMsg::Claim {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let transfer_nft_msg = Cw721ExecuteMsg::TransferNft {
            recipient: "sender".to_string(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        };
        // First message for royalty, Second message for tax
        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: MOCK_RATES_RECIPIENT.to_owned(),
                    amount: coins(10, "uusd"),
                }))
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: MOCK_RATES_RECIPIENT.to_owned(),
                    amount: coins(10, "uusd"),
                }))
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "owner".to_string(),
                    amount: coins(90, "uusd"),
                }))
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_TOKEN_ADDR.to_string(),
                    msg: encode_binary(&transfer_nft_msg).unwrap(),
                    funds: vec![],
                }))
                .add_attribute("action", "claim")
                .add_attribute("token_id", MOCK_UNCLAIMED_TOKEN)
                .add_attribute("token_contract", MOCK_TOKEN_ADDR)
                .add_attribute("recipient", "sender")
                .add_attribute("winning_bid_amount", Uint128::from(100u128))
                .add_attribute("auction_id", "1"),
            res
        );
    }

    #[test]
    fn execute_claim_auction_not_ended() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Claim {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionNotEnded {}, res.unwrap_err());
    }

    #[test]
    fn execute_claim_auction_already_claimed() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721HookMsg::StartAuction {
            start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
            end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
            coin_denom: "uusd".to_string(),
            whitelist: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: "claimed_token".to_string(),
            msg: encode_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Auction is over.
        env.block.time = Timestamp::from_seconds(300);

        let msg = ExecuteMsg::Claim {
            token_id: "claimed_token".to_string(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionAlreadyClaimed {}, res.unwrap_err());
    }

    #[test]
    fn execute_cancel_no_bids() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::CancelAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        // Auction start and end are 100 and 200.
        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_TOKEN_ADDR.to_owned(),
                msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: MOCK_TOKEN_OWNER.to_owned(),
                    token_id: MOCK_UNCLAIMED_TOKEN.to_owned()
                })
                .unwrap(),
                funds: vec![],
            })),
            res
        );

        assert!(
            TOKEN_AUCTION_STATE
                .load(deps.as_ref().storage, 1u128)
                .unwrap()
                .is_cancelled
        );
    }

    #[test]
    fn execute_cancel_with_bids() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };
        // Auction start and end are 100 and 200.
        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("bidder", &coins(100, "uusd"));
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::CancelAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_TOKEN_ADDR.to_owned(),
                    msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                        recipient: MOCK_TOKEN_OWNER.to_owned(),
                        token_id: MOCK_UNCLAIMED_TOKEN.to_owned()
                    })
                    .unwrap(),
                    funds: vec![],
                }))
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "bidder".to_string(),
                    amount: coins(100, "uusd")
                })),
            res
        );

        assert!(
            TOKEN_AUCTION_STATE
                .load(deps.as_ref().storage, 1u128)
                .unwrap()
                .is_cancelled
        );
    }

    #[test]
    fn execute_cancel_not_token_owner() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::CancelAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        // Auction start and end are 100 and 200.
        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("anyone", &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn execute_cancel_auction_ended() {
        let mut deps = mock_dependencies_custom(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg { modules: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::CancelAuction {
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_string(),
        };

        // Auction start and end are 100 and 200.
        env.block.time = Timestamp::from_seconds(300);

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionEnded {}, res.unwrap_err());
    }
}
