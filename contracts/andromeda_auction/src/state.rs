use andromeda_protocol::{
    auction::{AuctionStateResponse, Bid},
    common::OrderBy,
    error::ContractError,
    modules::{
        common::{calculate_fee, deduct_funds},
        hooks::PaymentAttribute,
    },
    rates::RateInfo,
};
use cosmwasm_std::{
    coin, Addr, Coin, DepsMut, Event, Order, StdError, StdResult, Storage, SubMsg, Uint128,
};
use cw721::Expiration;
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex, U128Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp;

const MAX_LIMIT: u64 = 30;
const DEFAULT_LIMIT: u64 = 10;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenAuctionState {
    pub start_time: Expiration,
    pub end_time: Expiration,
    pub high_bidder_addr: Addr,
    pub high_bidder_amount: Uint128,
    pub coin_denom: String,
    pub auction_id: Uint128,
    pub whitelist: Option<Vec<Addr>>,
    pub owner: String,
    pub token_id: String,
    pub token_address: String,
    pub is_cancelled: bool,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuctionInfo {
    pub auction_ids: Vec<Uint128>,
    pub token_address: String,
    pub token_id: String,
}

impl AuctionInfo {
    pub fn last(&self) -> Option<&Uint128> {
        self.auction_ids.last()
    }

    pub fn push(&mut self, e: Uint128) {
        self.auction_ids.push(e)
    }
}

impl From<TokenAuctionState> for AuctionStateResponse {
    fn from(token_auction_state: TokenAuctionState) -> AuctionStateResponse {
        AuctionStateResponse {
            start_time: token_auction_state.start_time,
            end_time: token_auction_state.end_time,
            high_bidder_addr: token_auction_state.high_bidder_addr.to_string(),
            high_bidder_amount: token_auction_state.high_bidder_amount,
            coin_denom: token_auction_state.coin_denom,
            auction_id: token_auction_state.auction_id,
            whitelist: token_auction_state.whitelist,
            is_cancelled: token_auction_state.is_cancelled,
        }
    }
}

pub const NEXT_AUCTION_ID: Item<Uint128> = Item::new("next_auction_id");

pub const BIDS: Map<U128Key, Vec<Bid>> = Map::new("bids"); // auction_id -> [bids]

pub const TOKEN_AUCTION_STATE: Map<U128Key, TokenAuctionState> = Map::new("auction_token_state");

pub const AUCTION_RATES: Item<Vec<RateInfo>> = Item::new("auction_rates");

pub struct AuctionIdIndices<'a> {
    /// (token_address, token_id + token_address)
    pub token: MultiIndex<'a, (String, Vec<u8>), AuctionInfo>,
}

impl<'a> IndexList<AuctionInfo> for AuctionIdIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AuctionInfo>> + '_> {
        let v: Vec<&dyn Index<AuctionInfo>> = vec![&self.token];
        Box::new(v.into_iter())
    }
}

pub fn auction_infos<'a>() -> IndexedMap<'a, &'a str, AuctionInfo, AuctionIdIndices<'a>> {
    let indexes = AuctionIdIndices {
        token: MultiIndex::new(
            |r, k| (r.token_address.clone(), k),
            "ownership",
            "token_index",
        ),
    };
    IndexedMap::new("ownership", indexes)
}

pub fn read_bids(
    storage: &dyn Storage,
    auction_id: U128Key,
    start_after: Option<u64>,
    limit: Option<u64>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<Bid>> {
    let mut bids = BIDS.load(storage, auction_id)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    // Passing in None implies we start from the beginning of the vector.
    let start = match start_after {
        None => 0,
        Some(x) => (x as usize) + 1usize,
    };

    // Start is INCLUSIVE, End is EXCLUSIVE
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Desc) => (
            bids.len() - cmp::min(bids.len(), start + limit),
            bids.len() - cmp::min(start, bids.len()),
            OrderBy::Desc,
        ),
        // Default ordering is Ascending.
        _ => (
            cmp::min(bids.len(), start),
            cmp::min(start + limit, bids.len()),
            OrderBy::Asc,
        ),
    };

    let slice = &mut bids[start..end];
    if order_by == OrderBy::Desc {
        slice.reverse();
    }

    Ok(slice.to_vec())
}

pub fn read_auction_infos(
    storage: &dyn Storage,
    token_address: String,
    start_after: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<AuctionInfo>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let keys: Vec<String> = auction_infos()
        .idx
        .token
        .prefix(token_address)
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|v| String::from_utf8(v.to_vec()))
        .collect::<Result<Vec<String>, _>>()
        .map_err(StdError::invalid_utf8)?;

    let mut res: Vec<AuctionInfo> = vec![];
    for key in keys.iter() {
        res.push(auction_infos().load(storage, key)?);
    }
    Ok(res)
}

pub fn calculate_additional_fees(
    deps: &DepsMut,
    payment_coin: Coin,
    rates: Vec<RateInfo>,
) -> Result<Option<Coin>, ContractError> {
    let mut additional_fees = coin(0, payment_coin.denom.clone());
    for rate_info in rates.iter() {
        if !rate_info.is_additive {
            continue;
        }
        let rate = rate_info.rate.validate(&deps.querier)?;
        let fee = calculate_fee(rate, &payment_coin)?;
        additional_fees.amount = additional_fees.amount.checked_add(
            fee.amount
                .checked_mul(Uint128::from(rate_info.receivers.len() as u128))?,
        )?;
    }

    if additional_fees.amount.is_zero() {
        Ok(None)
    } else {
        Ok(Some(additional_fees))
    }
}

type RequiredPayments = (Vec<Event>, Vec<SubMsg>, Vec<Coin>);

/**
 * Reused from rates contract and adjusted for auctions
 */
pub fn calculate_required_payments(
    deps: &DepsMut,
    coin: Coin,
    rates: Vec<RateInfo>,
) -> Result<RequiredPayments, ContractError> {
    let mut msgs: Vec<SubMsg> = vec![];
    let mut events: Vec<Event> = vec![];
    let mut leftover_funds = vec![coin.clone()];
    for rate_info in rates.iter() {
        let event_name = if rate_info.is_additive {
            "tax"
        } else {
            "royalty"
        };
        let mut event = Event::new(event_name);
        if let Some(desc) = &rate_info.description {
            event = event.add_attribute("description", desc);
        }
        let rate = rate_info.rate.validate(&deps.querier)?;
        let fee = calculate_fee(rate, &coin)?;
        for reciever in rate_info.receivers.iter() {
            if !rate_info.is_additive {
                deduct_funds(&mut leftover_funds, &fee)?;
                event = event.add_attribute("deducted", fee.to_string());
            }
            event = event.add_attribute(
                "payment",
                PaymentAttribute {
                    receiver: reciever.get_addr(),
                    amount: fee.clone(),
                }
                .to_string(),
            );
            let msg = reciever.generate_msg_native(deps.api, vec![fee.clone()])?;
            msgs.push(msg);
        }
        events.push(event);
    }

    Ok((events, msgs, leftover_funds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_protocol::communication::Recipient;
    use andromeda_protocol::modules::Rate;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{coin, Timestamp};

    fn get_mock_bids() -> Vec<Bid> {
        vec![
            Bid {
                bidder: "0".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
            Bid {
                bidder: "1".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
            Bid {
                bidder: "2".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
            Bid {
                bidder: "3".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
            Bid {
                bidder: "4".to_string(),
                amount: Uint128::zero(),
                timestamp: Timestamp::from_seconds(0),
            },
        ]
    }

    #[test]
    fn read_bids_no_params() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let bids = read_bids(deps.as_ref().storage, U128Key::new(0), None, None, None).unwrap();
        assert_eq!(get_mock_bids(), bids);
    }

    #[test]
    fn read_bids_no_params_desc() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let bids = read_bids(
            deps.as_ref().storage,
            U128Key::new(0),
            None,
            None,
            Some(OrderBy::Desc),
        )
        .unwrap();
        let mut expected_bids = get_mock_bids();
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(2),
                None,
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[0..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_limit() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                None,
                Some(2),
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[0..2], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[3..];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_limit() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(2),
                Some(1),
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..4], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[1..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_limit_too_high() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(2),
                Some(100),
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert_eq!(get_mock_bids()[3..], bids);

        let bids = func(OrderBy::Desc);
        let expected_bids = &mut get_mock_bids()[0..2];
        expected_bids.reverse();
        assert_eq!(expected_bids, bids);
    }

    #[test]
    fn read_bids_start_after_too_high() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(100),
                None,
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert!(bids.is_empty());

        let bids = func(OrderBy::Desc);
        assert!(bids.is_empty());
    }

    #[test]
    fn read_bids_start_after_and_limit_too_high() {
        let mut deps = mock_dependencies(&[]);

        BIDS.save(deps.as_mut().storage, U128Key::new(0), &get_mock_bids())
            .unwrap();

        let func = |order| {
            read_bids(
                deps.as_ref().storage,
                U128Key::new(0),
                Some(100),
                Some(100),
                Some(order),
            )
            .unwrap()
        };

        let bids = func(OrderBy::Asc);
        assert!(bids.is_empty());

        let bids = func(OrderBy::Desc);
        assert!(bids.is_empty());
    }

    #[test]
    fn test_calculate_required_payments() {
        let mut deps = mock_dependencies(&[]);
        let payment_amount = coin(100, "uluna");
        let recipient_one = Recipient::Addr(String::from("recipientone"));
        let recipient_two = Recipient::Addr(String::from("recipienttwo"));

        let empty_rates = Vec::<RateInfo>::new();
        let expected = (
            Vec::<Event>::new(),
            Vec::<SubMsg>::new(),
            vec![payment_amount.clone()],
        );
        let result =
            calculate_required_payments(&deps.as_mut(), payment_amount.clone(), empty_rates)
                .unwrap();

        assert_eq!(result, expected);

        let single_rate = vec![RateInfo {
            is_additive: false,
            receivers: vec![recipient_one.clone()],
            description: Some("Some royalty".to_string()),
            rate: Rate::Percent(Uint128::from(1u128)),
        }];
        let expected = (
            vec![Event::new("royalty")
                .add_attribute("description", String::from("Some royalty"))
                .add_attribute("deducted", coin(1, "uluna").to_string())
                .add_attribute(
                    "payment",
                    PaymentAttribute {
                        receiver: String::from("recipientone"),
                        amount: coin(1, "uluna"),
                    }
                    .to_string(),
                )],
            vec![recipient_one
                .generate_msg_native(mock_dependencies(&[]).as_mut().api, vec![coin(1, "uluna")])
                .unwrap()],
            vec![coin(99, "uluna")],
        );
        let result =
            calculate_required_payments(&deps.as_mut(), payment_amount.clone(), single_rate)
                .unwrap();

        assert_eq!(result, expected);

        let multi_rate = vec![
            RateInfo {
                is_additive: false,
                receivers: vec![recipient_one.clone()],
                description: Some("Some royalty".to_string()),
                rate: Rate::Percent(Uint128::from(1u128)),
            },
            RateInfo {
                is_additive: true,
                receivers: vec![recipient_two.clone()],
                description: Some("Some tax".to_string()),
                rate: Rate::Percent(Uint128::from(5u128)),
            },
        ];
        let expected = (
            vec![
                Event::new("royalty")
                    .add_attribute("description", String::from("Some royalty"))
                    .add_attribute("deducted", coin(1, "uluna").to_string())
                    .add_attribute(
                        "payment",
                        PaymentAttribute {
                            receiver: String::from("recipientone"),
                            amount: coin(1, "uluna"),
                        }
                        .to_string(),
                    ),
                Event::new("tax")
                    .add_attribute("description", String::from("Some tax"))
                    .add_attribute(
                        "payment",
                        PaymentAttribute {
                            receiver: String::from("recipienttwo"),
                            amount: coin(5, "uluna"),
                        }
                        .to_string(),
                    ),
            ],
            vec![
                recipient_one
                    .generate_msg_native(
                        mock_dependencies(&[]).as_mut().api,
                        vec![coin(1, "uluna")],
                    )
                    .unwrap(),
                recipient_two
                    .generate_msg_native(
                        mock_dependencies(&[]).as_mut().api,
                        vec![coin(5, "uluna")],
                    )
                    .unwrap(),
            ],
            vec![coin(99, "uluna")],
        );
        let result =
            calculate_required_payments(&deps.as_mut(), payment_amount, multi_rate).unwrap();

        assert_eq!(result, expected)
    }

    #[test]
    fn test_calculate_additional_fees() {
        let mut deps = mock_dependencies(&[]);
        let recipient_one = Recipient::Addr(String::from("recipientone"));
        let recipient_two = Recipient::Addr(String::from("recipienttwo"));
        let empty_rates = Vec::<RateInfo>::new();

        let resp =
            calculate_additional_fees(&deps.as_mut(), coin(100, "uluna"), empty_rates).unwrap();
        assert!(resp.is_none());

        let single_rate = vec![RateInfo {
            is_additive: true,
            receivers: vec![recipient_two.clone()],
            description: Some("Some tax".to_string()),
            rate: Rate::Percent(Uint128::from(5u128)),
        }];

        let resp =
            calculate_additional_fees(&deps.as_mut(), coin(100, "uluna"), single_rate).unwrap();
        assert!(resp.is_some());
        assert_eq!(coin(5, "uluna"), resp.unwrap());

        let multi_rate = vec![
            RateInfo {
                is_additive: false,
                receivers: vec![recipient_one.clone()],
                description: Some("Some royalty".to_string()),
                rate: Rate::Percent(Uint128::from(1u128)),
            },
            RateInfo {
                is_additive: true,
                receivers: vec![recipient_two, recipient_one],
                description: Some("Some tax".to_string()),
                rate: Rate::Percent(Uint128::from(5u128)),
            },
        ];

        let resp =
            calculate_additional_fees(&deps.as_mut(), coin(100, "uluna"), multi_rate).unwrap();
        assert!(resp.is_some());
        assert_eq!(coin(10, "uluna"), resp.unwrap())
    }
}
