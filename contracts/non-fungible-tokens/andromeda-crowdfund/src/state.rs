use andromeda_non_fungible_tokens::crowdfund::{
    CampaignConfig, CampaignStage, SimpleTierOrder, Tier, TierOrder,
};
use andromeda_std::error::ContractError;
use cosmwasm_std::{ensure, Addr, Order, Storage, Uint128, Uint64};
use cw_storage_plus::{Bound, Item, Map};

pub const CAMPAIGN_CONFIG: Item<CampaignConfig> = Item::new("campaign_config");

pub const CAMPAIGN_STAGE: Item<CampaignStage> = Item::new("campaign_stage");

pub const CURRENT_CAP: Item<Uint128> = Item::new("current_capital");

pub const TIERS: Map<u64, Tier> = Map::new("tiers");

pub const TIER_ORDERS: Map<(Addr, u64), u128> = Map::new("tier_orders");

pub const TIER_TOKEN_ID: Item<Uint128> = Item::new("tier_token_id");

pub(crate) fn update_config(
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

pub(crate) fn get_current_cap(storage: &dyn Storage) -> Uint128 {
    CURRENT_CAP.load(storage).unwrap_or_default()
}

pub(crate) fn set_current_cap(
    storage: &mut dyn Storage,
    current_cap: Uint128,
) -> Result<(), ContractError> {
    CURRENT_CAP
        .save(storage, &current_cap)
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

pub(crate) fn is_valid_tiers(storage: &mut dyn Storage) -> bool {
    !TIERS.is_empty(storage)
        && TIERS
            .range_raw(storage, None, None, Order::Ascending)
            .any(|res| res.unwrap().1.limit.is_none())
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
        let mut tier = TIERS.load(storage, new_order.level.into()).map_err(|_| {
            ContractError::InvalidTier {
                operation: "set_tier_orders".to_string(),
                msg: format!("Tier with level {} does not exist", new_order.level),
            }
        })?;
        if let Some(limit) = tier.limit {
            tier.sold_amount = tier.sold_amount.checked_add(new_order.amount)?;
            ensure!(
                limit >= tier.sold_amount,
                ContractError::PurchaseLimitReached {}
            );

            update_tier(storage, &tier)?;
        }

        let mut order = TIER_ORDERS
            .load(storage, (new_order.orderer.clone(), new_order.level.into()))
            .unwrap_or(0);
        order += new_order.amount.u128();
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
) -> Result<Vec<SimpleTierOrder>, ContractError> {
    let limit = limit.unwrap_or(u32::MAX) as usize;
    let start = start_after.map(Bound::exclusive);

    TIER_ORDERS
        .prefix(user)
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|v| {
            let (level, amount) = v?;
            Ok(SimpleTierOrder {
                level: Uint64::new(level),
                amount: Uint128::new(amount),
            })
        })
        .collect()
}

pub(crate) fn get_and_increase_tier_token_id(
    storage: &mut dyn Storage,
) -> Result<Uint128, ContractError> {
    let tier_token_id = TIER_TOKEN_ID.load(storage)?;
    let next_tier_token_id = tier_token_id.checked_add(Uint128::from(1u128))?;
    TIER_TOKEN_ID.save(storage, &next_tier_token_id)?;
    Ok(tier_token_id)
}
