use andromeda_non_fungible_tokens::crowdfund::{
    CampaignConfig, CampaignStage, SimpleTierOrder, Tier, TierOrder, TierResponseItem,
};
use andromeda_std::{
    common::{MillisecondsExpiration, OrderBy},
    error::ContractError,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Order, Storage, Uint128, Uint64};
use cw_storage_plus::{Bound, Item, Map};

#[cw_serde]
pub struct Duration {
    /// Time when campaign starts
    pub start_time: Option<MillisecondsExpiration>,
    /// Time when campaign ends
    pub end_time: MillisecondsExpiration,
}
pub const CAMPAIGN_DURATION: Item<Duration> = Item::new("campaign_duration");

pub const CAMPAIGN_CONFIG: Item<CampaignConfig> = Item::new("campaign_config");

pub const CAMPAIGN_STAGE: Item<CampaignStage> = Item::new("campaign_stage");

pub const CURRENT_CAPITAL: Item<Uint128> = Item::new("current_capital");

pub const TIERS: Map<u64, Tier> = Map::new("tiers");

pub const TIER_SALES: Map<u64, Uint128> = Map::new("tier_sales");

pub const TIER_ORDERS: Map<(Addr, u64), OrderInfo> = Map::new("tier_orders");

pub const TIER_TOKEN_ID: Item<Uint128> = Item::new("tier_token_id");

#[cw_serde]
#[derive(Default)]
pub struct OrderInfo {
    pub ordered: u128,
    pub preordered: u128,
}

impl OrderInfo {
    pub fn amount(self) -> Option<u128> {
        self.ordered.checked_add(self.preordered)
    }
}

pub(crate) fn set_config(
    storage: &mut dyn Storage,
    config: CampaignConfig,
) -> Result<(), ContractError> {
    CAMPAIGN_CONFIG
        .save(storage, &config)
        .map_err(ContractError::Std)
}

pub(crate) fn get_config(storage: &dyn Storage) -> Result<CampaignConfig, ContractError> {
    CAMPAIGN_CONFIG.load(storage).map_err(ContractError::Std)
}

pub(crate) fn get_duration(storage: &dyn Storage) -> Result<Duration, ContractError> {
    CAMPAIGN_DURATION.load(storage).map_err(ContractError::Std)
}

pub(crate) fn set_duration(
    storage: &mut dyn Storage,
    duration: Duration,
) -> Result<(), ContractError> {
    CAMPAIGN_DURATION
        .save(storage, &duration)
        .map_err(ContractError::Std)
}

pub(crate) fn get_current_capital(storage: &dyn Storage) -> Uint128 {
    CURRENT_CAPITAL.load(storage).unwrap_or_default()
}

pub(crate) fn set_current_capital(
    storage: &mut dyn Storage,
    current_capital: Uint128,
) -> Result<(), ContractError> {
    CURRENT_CAPITAL
        .save(storage, &current_capital)
        .map_err(ContractError::Std)
}

/// Only used on the instantiation
pub(crate) fn set_tiers(storage: &mut dyn Storage, tiers: Vec<Tier>) -> Result<(), ContractError> {
    for tier in tiers {
        tier.validate()?;
        ensure!(
            !TIERS.has(storage, tier.level.into()),
            ContractError::InvalidTier {
                operation: "instantiate".to_string(),
                msg: format!("Tier with level {} already defined", tier.level)
            }
        );
        TIERS.save(storage, tier.level.into(), &tier)?;
    }

    Ok(())
}

pub(crate) fn get_tier(storage: &mut dyn Storage, level: u64) -> Result<Tier, ContractError> {
    TIERS
        .load(storage, level)
        .map_err(|_| ContractError::InvalidTier {
            operation: "get_tier".to_string(),
            msg: format!("Tier with level {} does not exist", level),
        })
}

pub(crate) fn add_tier(storage: &mut dyn Storage, tier: &Tier) -> Result<(), ContractError> {
    ensure!(
        !TIERS.has(storage, tier.level.into()),
        ContractError::InvalidTier {
            operation: "add".to_string(),
            msg: format!("Tier with level {} already exist", tier.level)
        }
    );
    TIERS.save(storage, tier.level.into(), tier)?;
    Ok(())
}

pub(crate) fn update_tier(storage: &mut dyn Storage, tier: &Tier) -> Result<(), ContractError> {
    ensure!(
        TIERS.has(storage, tier.level.into()),
        ContractError::InvalidTier {
            operation: "update".to_string(),
            msg: format!("Tier with level {} does not exist", tier.level),
        }
    );

    TIERS.save(storage, tier.level.into(), tier)?;
    Ok(())
}

pub(crate) fn remove_tier(storage: &mut dyn Storage, level: u64) -> Result<(), ContractError> {
    ensure!(
        TIERS.has(storage, level),
        ContractError::InvalidTier {
            operation: "remove".to_string(),
            msg: format!("Tier with level {} does not exist", level)
        }
    );

    TIERS.remove(storage, level);
    Ok(())
}

pub(crate) fn set_tier_sales(
    storage: &mut dyn Storage,
    level: u64,
    sold_amount: Uint128,
) -> Result<(), ContractError> {
    TIER_SALES.save(storage, level, &sold_amount)?;
    Ok(())
}

pub(crate) fn get_tiers(
    storage: &dyn Storage,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> Result<Vec<TierResponseItem>, ContractError> {
    let limit = limit.unwrap_or(u32::MAX) as usize;
    let start = start_after.map(Bound::exclusive);
    let order_by = match order_by.unwrap_or(OrderBy::Desc) {
        OrderBy::Desc => Order::Descending,
        _ => Order::Ascending,
    };

    TIERS
        .range(storage, start, None, order_by)
        .take(limit)
        .map(|v| {
            let (level, tier) = v?;
            let sold_amount = TIER_SALES.may_load(storage, level)?.unwrap_or_default();
            Ok(TierResponseItem { tier, sold_amount })
        })
        .collect()
}

pub(crate) fn is_valid_tiers(storage: &mut dyn Storage) -> bool {
    !TIERS.is_empty(storage)
}

pub(crate) fn get_current_stage(storage: &dyn Storage) -> CampaignStage {
    CAMPAIGN_STAGE.load(storage).unwrap_or(CampaignStage::READY)
}

pub(crate) fn set_current_stage(
    storage: &mut dyn Storage,
    stage: CampaignStage,
) -> Result<(), ContractError> {
    CAMPAIGN_STAGE
        .save(storage, &stage)
        .map_err(ContractError::Std)
}

pub(crate) fn set_tier_orders(
    storage: &mut dyn Storage,
    orders: Vec<TierOrder>,
) -> Result<(), ContractError> {
    for new_order in orders {
        let tier = TIERS.load(storage, new_order.level.into()).map_err(|_| {
            ContractError::InvalidTier {
                operation: "set_tier_orders".to_string(),
                msg: format!("Tier with level {} does not exist", new_order.level),
            }
        })?;

        let mut sold_amount = TIER_SALES
            .load(storage, new_order.level.into())
            .unwrap_or_default();
        sold_amount = sold_amount.checked_add(new_order.amount)?;
        if let Some(limit) = tier.limit {
            ensure!(limit >= sold_amount, ContractError::PurchaseLimitReached {});
        }
        update_tier(storage, &tier)?;
        set_tier_sales(storage, new_order.level.into(), sold_amount)?;
        let mut order = TIER_ORDERS
            .load(storage, (new_order.orderer.clone(), new_order.level.into()))
            .unwrap_or_default();

        if new_order.is_presale {
            order.preordered += new_order.amount.u128();
        } else {
            order.ordered += new_order.amount.u128();
        }

        TIER_ORDERS.save(
            storage,
            (new_order.orderer.clone(), new_order.level.into()),
            &order,
        )?;
    }
    Ok(())
}

pub(crate) fn get_user_orders(
    storage: &dyn Storage,
    user: Addr,
    start_after: Option<u64>,
    limit: Option<u32>,
    include_presale: bool,
    order_by: Option<OrderBy>,
) -> Vec<SimpleTierOrder> {
    let limit = limit.unwrap_or(u32::MAX) as usize;
    let start = start_after.map(Bound::exclusive);
    let order_by = match order_by.unwrap_or(OrderBy::Desc) {
        OrderBy::Desc => Order::Descending,
        _ => Order::Ascending,
    };

    TIER_ORDERS
        .prefix(user)
        .range(storage, start, None, order_by)
        .take(limit)
        .map(|v| {
            let (level, order_info) = v.unwrap();
            let amount = if include_presale {
                order_info.amount().unwrap()
            } else {
                order_info.ordered
            };
            SimpleTierOrder {
                level: Uint64::new(level),
                amount: Uint128::new(amount),
            }
        })
        .collect()
}

pub(crate) fn clear_user_orders(
    storage: &mut dyn Storage,
    user: Addr,
) -> Result<(), ContractError> {
    let levels: Vec<u64> = TIER_ORDERS
        .prefix(user.clone())
        .range(storage, None, None, Order::Ascending)
        .map(|v| v.unwrap().0)
        .collect();

    for level in levels {
        TIER_ORDERS.remove(storage, (user.clone(), level));
    }
    Ok(())
}

pub(crate) fn get_and_increase_tier_token_id(
    storage: &mut dyn Storage,
) -> Result<Uint128, ContractError> {
    let tier_token_id = TIER_TOKEN_ID.load(storage).unwrap_or_default();
    let next_tier_token_id = tier_token_id.checked_add(Uint128::one())?;
    TIER_TOKEN_ID.save(storage, &next_tier_token_id)?;
    Ok(tier_token_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use andromeda_non_fungible_tokens::{crowdfund::TierMetaData, cw721::TokenExtension};
    use cosmwasm_std::testing::MockStorage;

    fn mock_storage() -> MockStorage {
        let mut storage = MockStorage::new();
        // Initialize some mock data for testing
        let tiers = vec![
            Tier {
                level: Uint64::one(),
                price: Uint128::new(100),
                limit: Some(Uint128::new(1000)),
                label: "tier 1".to_string(),
                metadata: TierMetaData {
                    token_uri: None,
                    extension: TokenExtension {
                        ..Default::default()
                    },
                },
            },
            Tier {
                level: Uint64::new(2u64),
                price: Uint128::new(200),
                limit: None,
                label: "tier 2".to_string(),
                metadata: TierMetaData {
                    token_uri: None,
                    extension: TokenExtension {
                        ..Default::default()
                    },
                },
            },
        ];
        set_tiers(&mut storage, tiers).unwrap();
        let user1 = Addr::unchecked("user1");
        let orders = vec![
            TierOrder {
                orderer: user1.clone(),
                level: Uint64::one(),
                amount: Uint128::new(50),
                is_presale: true,
            },
            TierOrder {
                orderer: user1.clone(),
                level: Uint64::one(),
                amount: Uint128::new(50),
                is_presale: false,
            },
        ];
        set_tier_orders(&mut storage, orders).unwrap();
        storage
    }

    pub struct GetUserOrderTestCase {
        name: String,
        include_presale: bool,
        user: Addr,
        expected_res: Vec<SimpleTierOrder>,
    }
    fn get_tier_sales(storage: &mut dyn Storage, level: u64) -> Uint128 {
        TIER_SALES.load(storage, level).unwrap_or_default()
    }

    #[test]
    fn test_get_user_orders() {
        let test_cases = vec![
            GetUserOrderTestCase {
                name: "get_user_orders including pesale".to_string(),
                include_presale: true,
                user: Addr::unchecked("user1"),
                expected_res: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(100),
                }],
            },
            GetUserOrderTestCase {
                name: "get_user_orders excluding pesale".to_string(),
                include_presale: false,
                user: Addr::unchecked("user1"),
                expected_res: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(50),
                }],
            },
            GetUserOrderTestCase {
                name: "get_user_orders for non ordered user".to_string(),
                include_presale: false,
                user: Addr::unchecked("user2"),
                expected_res: vec![],
            },
        ];
        let storage = mock_storage();

        for test in test_cases {
            let res = get_user_orders(
                &storage,
                test.user.clone(),
                None,
                None,
                test.include_presale,
                None,
            );
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
        }
    }

    pub struct SetOrderTestCase {
        name: String,
        order: TierOrder,
        expected_res: Result<(), ContractError>,
    }

    #[test]
    fn test_set_tier_orders() {
        let test_cases = vec![
            SetOrderTestCase {
                name: "set_tier_orders with valid orders".to_string(),
                order: TierOrder {
                    level: Uint64::new(1),
                    amount: Uint128::new(100),
                    orderer: Addr::unchecked("user1"),
                    is_presale: false,
                },
                expected_res: Ok(()),
            },
            SetOrderTestCase {
                name: "set_tier_orders with an order exceeding limit".to_string(),
                order: TierOrder {
                    level: Uint64::new(1),
                    amount: Uint128::new(1000),
                    orderer: Addr::unchecked("user2"),
                    is_presale: false,
                },
                expected_res: Err(ContractError::PurchaseLimitReached {}),
            },
            SetOrderTestCase {
                name: "set_tier_orders with an order for non-existing tier".to_string(),
                order: TierOrder {
                    level: Uint64::new(3),
                    amount: Uint128::new(50),
                    orderer: Addr::unchecked("user3"),
                    is_presale: false,
                },
                expected_res: Err(ContractError::InvalidTier {
                    operation: "set_tier_orders".to_string(),
                    msg: "Tier with level 3 does not exist".to_string(),
                }),
            },
        ];

        for test in test_cases {
            let mut storage = mock_storage();
            let level: u64 = test.order.level.into();
            let ordered_amount = test.order.amount;
            let orderer = test.order.orderer.clone();
            let prev_sold_amount = get_tier_sales(&mut storage, level);
            let prev_order = TIER_ORDERS.load(&storage, (orderer.clone(), level));
            let is_presale = test.order.is_presale;

            let res = set_tier_orders(&mut storage, vec![test.order]);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);

            if res.is_ok() {
                let sold_amount = get_tier_sales(&mut storage, level);
                assert_eq!(sold_amount, prev_sold_amount + ordered_amount);
                let order = TIER_ORDERS.load(&storage, (orderer, level)).unwrap();
                let prev_order = prev_order.unwrap();
                if is_presale {
                    assert_eq!(
                        order,
                        OrderInfo {
                            ordered: prev_order.ordered,
                            preordered: prev_order.preordered + ordered_amount.u128(),
                        },
                        "Test case: {}",
                        test.name
                    );
                } else {
                    assert_eq!(
                        order,
                        OrderInfo {
                            ordered: prev_order.ordered + ordered_amount.u128(),
                            preordered: prev_order.preordered,
                        },
                        "Test case: {}",
                        test.name
                    );
                }
            }
        }
    }
}
