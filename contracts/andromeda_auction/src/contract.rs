use crate::state::{
    Bid, Config, TokenAuctionState, AUCTION_IDS, BIDS, CONFIG, NEXT_AUCTION_ID, TOKEN_AUCTION_STATE,
};
use andromeda_protocol::{
    auction::{AuctionStateResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    common::get_tax_deducted_funds,
    error::ContractError,
    require,
    token::{ExecuteMsg as TokenExecuteMsg, QueryMsg as TokenQueryMsg},
};
use cosmwasm_std::{
    attr, coins, entry_point, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Storage, SubMsg,
    Uint128, WasmMsg, WasmQuery,
};
use cw721::OwnerOfResponse;
use cw_storage_plus::U128Key;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config: Config = Config {
        token_addr: msg.token_addr.clone(),
    };
    NEXT_AUCTION_ID.save(deps.storage, &Uint128::from(0u128))?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("token_addr", msg.token_addr))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::PlaceBid { token_id } => execute_place_bid(deps, env, info, token_id),
        ExecuteMsg::ClaimReward { token_id } => execute_claim_reward(deps, env, info, token_id),
        ExecuteMsg::Withdraw { auction_id } => execute_withdraw(deps, env, info, auction_id),
        ExecuteMsg::StartAuction {
            token_id,
            start_time,
            end_time,
            coin_denom,
        } => execute_start_auction(deps, env, info, token_id, start_time, end_time, coin_denom),
    }
}

pub fn execute_place_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    require(
        info.funds.len() <= 1usize,
        ContractError::MoreThanOneCoinSent {},
    )?;
    let config = CONFIG.load(deps.storage)?;
    let mut token_auction_state = get_existing_token_auction_state(&deps.as_ref(), &token_id)?;

    require(
        token_auction_state.high_bidder_addr != info.sender.to_string(),
        ContractError::HighestBidderCannotOutBid {},
    )?;
    require(
        env.block.time.seconds() >= token_auction_state.start_time,
        ContractError::AuctionNotStarted {},
    )?;
    require(
        env.block.time.seconds() < token_auction_state.end_time,
        ContractError::AuctionEnded {},
    )?;

    let coin_denom = token_auction_state.coin_denom.clone();
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to auction", coin_denom))
        })?;

    let mut bids = BIDS.load(
        deps.storage,
        U128Key::new(token_auction_state.auction_id.u128()),
    )?;
    let existing_bid_option = bids
        .iter_mut()
        .find(|bid| bid.bidder == info.sender.to_string());

    let mut bid = Bid {
        bidder: info.sender.to_string(),
        amount: Uint128::zero(),
    };
    if let Some(existing_bid) = existing_bid_option {
        bid.amount = existing_bid.amount;
    }
    bid.amount = bid.amount.checked_add(payment.amount)?;
    require(
        token_auction_state.high_bidder_amount < bid.amount,
        ContractError::BidSmallerThanHighestBid {},
    )?;

    let token_owner_res = query_owner_of(deps.querier, config.token_addr, token_id.clone())?;
    require(
        token_owner_res.owner != info.sender,
        ContractError::TokenOwnerCannotBid {},
    )?;
    token_auction_state.high_bidder_addr = info.sender.clone();
    token_auction_state.high_bidder_amount = bid.amount;

    let key = U128Key::new(token_auction_state.auction_id.u128());
    TOKEN_AUCTION_STATE.save(deps.storage, key.clone(), &token_auction_state)?;
    let mut bids_for_auction = BIDS.load(deps.storage, key.clone())?;
    bids_for_auction.push(bid);
    BIDS.save(deps.storage, key, &bids_for_auction)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "bid"),
        attr("token_id", token_id.clone()),
        attr("bider", info.sender.to_string()),
        attr("amount", payment.amount.to_string()),
    ]))
}

pub fn execute_claim_reward(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut token_auction_state = get_existing_token_auction_state(&deps.as_ref(), &token_id)?;
    require(
        !token_auction_state.reward_claimed,
        ContractError::AuctionRewardAlreadyClaimed {},
    )?;
    require(
        env.block.time.seconds() > token_auction_state.end_time,
        ContractError::AuctionNotEnded {},
    )?;
    let tax_deducted_funds = get_tax_deducted_funds(
        &deps,
        coins(
            token_auction_state.high_bidder_amount.u128(),
            token_auction_state.coin_denom.clone(),
        ),
    )?;
    let transfer_agreement_msg = TokenExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        denom: token_auction_state.coin_denom.clone(),
        purchaser: env.contract.address.to_string(),
        amount: tax_deducted_funds[0].amount,
    };
    let transfer_nft_msg = TokenExecuteMsg::TransferNft {
        recipient: token_auction_state.high_bidder_addr.to_string(),
        token_id: token_id.clone(),
    };
    token_auction_state.reward_claimed = true;
    TOKEN_AUCTION_STATE.save(
        deps.storage,
        U128Key::new(token_auction_state.auction_id.u128()),
        &token_auction_state,
    )?;

    Ok(Response::new()
        .add_submessage(SubMsg::new(WasmMsg::Execute {
            contract_addr: config.token_addr.to_string(),
            msg: to_binary(&transfer_agreement_msg)?,
            funds: vec![],
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.token_addr.to_string(),
            msg: to_binary(&transfer_nft_msg)?,
            funds: tax_deducted_funds,
        }))
        .add_attribute("action", "claim_reward")
        .add_attribute("token_id", token_id)
        .add_attribute("token_contract", config.token_addr)
        .add_attribute("recipient", &token_auction_state.high_bidder_addr)
        .add_attribute("winning_bid_amount", token_auction_state.high_bidder_amount)
        .add_attribute("auction_id", token_auction_state.auction_id))
}

pub fn execute_start_auction(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    start_time: u64,
    end_time: u64,
    coin_denom: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let owner_of_response = query_owner_of(deps.querier, config.token_addr, token_id.clone())?;
    if owner_of_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if start_time >= end_time {
        return Err(ContractError::StartTimeAfterEndTime {});
    }
    if start_time <= env.block.time.seconds() {
        return Err(ContractError::StartTimeInThePast {});
    }

    let latest_auction_id: Uint128 = match AUCTION_IDS.may_load(deps.storage, &token_id)? {
        None => get_and_increment_next_auction_id(deps.storage, &token_id)?,
        Some(auction_ids) => {
            // If the vec exists there will always be at least one element so unwrapping is fine.
            let latest_auction_id = *auction_ids.last().unwrap();
            let token_auction_state =
                TOKEN_AUCTION_STATE.load(deps.storage, U128Key::new(latest_auction_id.u128()))?;
            if env.block.time.seconds() < token_auction_state.start_time {
                token_auction_state.auction_id
            } else {
                // Previous auction must be completed before new auction can start.
                require(
                    token_auction_state.reward_claimed,
                    ContractError::AuctionNotEnded {},
                )?;
                get_and_increment_next_auction_id(deps.storage, &token_id)?
            }
        }
    };
    BIDS.save(
        deps.storage,
        U128Key::new(latest_auction_id.u128()),
        &vec![],
    )?;

    TOKEN_AUCTION_STATE.save(
        deps.storage,
        U128Key::new(latest_auction_id.u128()),
        &TokenAuctionState {
            start_time,
            end_time,
            high_bidder_addr: Addr::unchecked(""),
            high_bidder_amount: Uint128::zero(),
            coin_denom,
            auction_id: latest_auction_id,
            reward_claimed: false,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_auction"),
        attr("start_time", start_time.to_string()),
        attr("end_time", end_time.to_string()),
    ]))
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auction_id: Uint128,
) -> Result<Response, ContractError> {
    let token_auction_state =
        TOKEN_AUCTION_STATE.load(deps.storage, U128Key::new(auction_id.u128()))?;

    require(
        env.block.time.seconds() > token_auction_state.start_time,
        ContractError::AuctionNotStarted {},
    )?;

    require(
        info.sender != token_auction_state.high_bidder_addr,
        ContractError::CannotWithdrawHighestBid {},
    )?;

    let funds_by_bidder_token = BIDS.load(deps.storage, U128Key::new(auction_id.u128()))?;
    let funds_by_bidder_option = funds_by_bidder_token
        .iter()
        // rfind ensures that we get the latest bid as the caller may have made multiple bids.
        .rfind(|bid| bid.bidder == info.sender.to_string());

    let withdraw_amount = if let Some(funds_by_bidder) = funds_by_bidder_option {
        funds_by_bidder.amount
    } else {
        Uint128::zero()
    };

    require(
        !withdraw_amount.is_zero(),
        ContractError::WithdrawalIsEmpty {},
    )?;
    let tax_deducted_funds = get_tax_deducted_funds(
        &deps,
        coins(withdraw_amount.u128(), token_auction_state.coin_denom),
    )?;

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: tax_deducted_funds,
        }))
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("auction_id", token_auction_state.auction_id),
            attr("receiver", info.sender),
            attr("withdraw_amount", withdraw_amount),
        ]))
}

fn get_existing_token_auction_state(
    deps: &Deps,
    token_id: &str,
) -> Result<TokenAuctionState, ContractError> {
    let latest_auction_id: Uint128 = match AUCTION_IDS.may_load(deps.storage, &token_id)? {
        None => {
            println!("Error before");
            return Err(ContractError::AuctionDoesNotExist {});
        }
        Some(auction_ids) => *auction_ids.last().unwrap(),
    };
    let token_auction_state =
        TOKEN_AUCTION_STATE.may_load(deps.storage, U128Key::new(latest_auction_id.u128()))?;
    if let Some(token_auction_state) = token_auction_state {
        return Ok(token_auction_state);
    }
    println!("Error here");
    return Err(ContractError::AuctionDoesNotExist {});
}

fn get_and_increment_next_auction_id(
    storage: &mut dyn Storage,
    token_id: &str,
) -> Result<Uint128, ContractError> {
    let next_auction_id = NEXT_AUCTION_ID.load(storage)?;
    let incremented_next_auction_id = next_auction_id.checked_add(Uint128::from(1u128))?;
    NEXT_AUCTION_ID.save(storage, &incremented_next_auction_id)?;
    let mut auction_ids = match AUCTION_IDS.may_load(storage, token_id)? {
        None => vec![],
        Some(vec) => vec,
    };
    auction_ids.push(next_auction_id);
    AUCTION_IDS.save(storage, token_id, &auction_ids)?;
    Ok(next_auction_id)
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LatestAuctionState { token_id } => {
            to_binary(&query_get_latest_auction_state(deps, token_id)?)
        }
    }
}

fn query_get_latest_auction_state(deps: Deps, token_id: String) -> StdResult<AuctionStateResponse> {
    let token_auction_state_result = get_existing_token_auction_state(&deps, &token_id);
    if let Ok(token_auction_state) = token_auction_state_result {
        return Ok(AuctionStateResponse {
            start_time: token_auction_state.start_time,
            end_time: token_auction_state.end_time,
            high_bidder_addr: token_auction_state.high_bidder_addr.to_string(),
            high_bidder_amount: token_auction_state.high_bidder_amount,
            reward_claimed: token_auction_state.reward_claimed,
            coin_denom: token_auction_state.coin_denom,
            auction_id: token_auction_state.auction_id,
        });
    }
    Err(StdError::NotFound {
        kind: "TokenAuctionState".to_string(),
    })
}

fn query_owner_of(
    querier: QuerierWrapper,
    token_addr: String,
    token_id: String,
) -> StdResult<OwnerOfResponse> {
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_addr,
        msg: to_binary(&TokenQueryMsg::OwnerOf { token_id })?,
    }))?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::{mock_dependencies_custom, MOCK_TOKEN_ADDR, MOCK_TOKEN_OWNER};
    use andromeda_protocol::auction::{ExecuteMsg, InstantiateMsg};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{
        attr, coins, from_binary, BankMsg, CosmosMsg, Decimal, Response, Timestamp,
    };

    fn query_latest_auction_state_helper(deps: Deps, env: Env) -> AuctionStateResponse {
        let query_msg = QueryMsg::LatestAuctionState {
            token_id: "token_001".to_string(),
        };
        from_binary(&query(deps, env.clone(), query_msg).unwrap()).unwrap()
    }

    #[test]
    fn test_auction_instantiate() {
        let owner = "creator";
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info(owner, &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_place_bid_non_existing_auction() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };
        let info = mock_info(MOCK_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::AuctionDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_auction_not_started() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100u64,
            end_time: 200u64,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(50u64);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::AuctionNotStarted {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_auction_ended() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(300);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::AuctionEnded {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_token_owner_cannot_bid() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info(MOCK_TOKEN_OWNER, &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::TokenOwnerCannotBid {}, res.unwrap_err());
    }
    #[test]
    fn execute_place_bid_highest_bidder_cannot_outbid() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("sender", &coins(200, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(
            ContractError::HighestBidderCannotOutBid {},
            res.unwrap_err()
        );
    }

    #[test]
    fn execute_place_bid_bid_smaller_than_highest_bid() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("other", &coins(50, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::BidSmallerThanHighestBid {}, res.unwrap_err());
    }

    #[test]
    fn execute_place_bid_update_existing_bid() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "bid"),
                attr("token_id", "token_001"),
                attr("bider", info.sender),
                attr("amount", "100"),
            ]),
        );
        let mut expected_response = AuctionStateResponse {
            start_time: 100,
            end_time: 200,
            high_bidder_addr: "sender".to_string(),
            high_bidder_amount: Uint128::from(100u128),
            auction_id: Uint128::zero(),
            coin_denom: "uusd".to_string(),
            reward_claimed: false,
        };

        let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        assert_eq!(expected_response, res);

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("other", &coins(200, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        expected_response.high_bidder_addr = "other".to_string();
        expected_response.high_bidder_amount = Uint128::from(200u128);
        let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        assert_eq!(expected_response, res);

        env.block.time = Timestamp::from_seconds(170);
        let info = mock_info("sender", &coins(150, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        expected_response.high_bidder_addr = "sender".to_string();
        expected_response.high_bidder_amount = Uint128::from(250u128);
        let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        assert_eq!(expected_response, res);
    }

    #[test]
    fn execute_start_auction() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "start_auction"),
                attr("start_time", "100"),
                attr("end_time", "200"),
            ]),
        );
    }

    #[test]
    fn execute_start_auction_start_time_in_past() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(150);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::StartTimeInThePast {}, res.unwrap_err());
    }

    #[test]
    fn execute_start_auction_start_time_after_end_time() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 300,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::StartTimeAfterEndTime {}, res.unwrap_err());
    }

    #[test]
    fn execute_start_auction_not_token_owner() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info("not_owner", &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn execute_start_auction_update_not_started_auction() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        assert_eq!(100, res.start_time);
        assert_eq!(200, res.end_time);
        assert_eq!(Uint128::from(0u128), res.auction_id);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 150,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        assert_eq!(150, res.start_time);
        assert_eq!(200, res.end_time);
        assert_eq!(Uint128::from(0u128), res.auction_id);
    }

    #[test]
    fn execute_start_auction_cannot_update_started_auction() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 150,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        env.block.time = Timestamp::from_seconds(120);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::AuctionNotEnded {}, res.unwrap_err());
    }

    #[test]
    fn execute_withdraw_auction_not_started() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::Withdraw {
            auction_id: Uint128::from(0u128),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::AuctionNotStarted {}, res.unwrap_err());
    }

    #[test]
    fn execute_withdraw_cannot_withdraw_highest_bid() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(150);
        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::Withdraw {
            auction_id: Uint128::from(0u128),
        };
        env.block.time = Timestamp::from_seconds(160);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::CannotWithdrawHighestBid {}, res.unwrap_err());
    }

    #[test]
    fn execute_withdraw() {
        let mut deps = mock_dependencies_custom(&[]);
        deps.querier.with_tax(
            Decimal::percent(10),
            &[(&"uusd".to_string(), &Uint128::from(1500000u128))],
        );
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(150);
        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("other", &coins(150, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::Withdraw {
            auction_id: Uint128::from(0u128),
        };
        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("sender", &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: coins(90, "uusd")
                }))
                .add_attributes(vec![
                    attr("action", "withdraw"),
                    attr("auction_id", Uint128::from(0u128)),
                    attr("receiver", info.sender.to_string()),
                    attr("withdraw_amount", "100".to_string())
                ]),
            res
        );
    }

    #[test]
    fn execute_claim_reward() {
        let mut deps = mock_dependencies_custom(&[]);
        deps.querier.with_tax(
            Decimal::percent(10),
            &[(&"uusd".to_string(), &Uint128::from(1500000u128))],
        );
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(250);

        let msg = ExecuteMsg::ClaimReward {
            token_id: "token_001".to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let transfer_agreement_msg = TokenExecuteMsg::TransferAgreement {
            token_id: "token_001".to_string(),
            denom: "uusd".to_string(),
            purchaser: env.contract.address.to_string(),
            amount: Uint128::from(90u128),
        };
        let transfer_nft_msg = TokenExecuteMsg::TransferNft {
            recipient: "sender".to_string(),
            token_id: "token_001".to_string(),
        };
        assert_eq!(
            Response::new()
                .add_submessage(SubMsg::new(WasmMsg::Execute {
                    contract_addr: MOCK_TOKEN_ADDR.to_string(),
                    msg: to_binary(&transfer_agreement_msg).unwrap(),
                    funds: vec![],
                }))
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_TOKEN_ADDR.to_string(),
                    msg: to_binary(&transfer_nft_msg).unwrap(),
                    funds: coins(90, "uusd",),
                }))
                .add_attribute("action", "claim_reward")
                .add_attribute("token_id", "token_001")
                .add_attribute("token_contract", MOCK_TOKEN_ADDR)
                .add_attribute("recipient", "sender")
                .add_attribute("winning_bid_amount", Uint128::from(100u128))
                .add_attribute("auction_id", Uint128::zero()),
            res
        );
    }

    #[test]
    fn execute_claim_reward_auction_not_ended() {
        let mut deps = mock_dependencies_custom(&[]);
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            token_addr: MOCK_TOKEN_ADDR.to_string(),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let info = mock_info(MOCK_TOKEN_OWNER, &[]);

        let msg = ExecuteMsg::StartAuction {
            token_id: "token_001".to_string(),
            start_time: 100,
            end_time: 200,
            coin_denom: "uusd".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: "token_001".to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "uusd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        let msg = ExecuteMsg::ClaimReward {
            token_id: "token_001".to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        assert_eq!(ContractError::AuctionNotEnded {}, res.unwrap_err());
    }
}
