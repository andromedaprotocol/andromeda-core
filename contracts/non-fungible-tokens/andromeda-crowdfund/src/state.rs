use andromeda_non_fungible_tokens::crowdfund::{CampaignConfig, CampaignStage, Tier};
use andromeda_std::error::ContractError;
use cosmwasm_std::{ensure, Storage, Uint128};
use cw_storage_plus::{Item, Map};

pub const CAMPAIGN_CONFIG: Item<CampaignConfig> = Item::new("campaign_config");

pub const CAMPAIGN_STAGE: Item<CampaignStage> = Item::new("campaign_stage");

pub const CURRENT_CAP: Item<Uint128> = Item::new("current_capital");

pub const TIERS: Map<u64, Tier> = Map::new("tiers");

pub(crate) fn update_config(
    storage: &mut dyn Storage,
    config: CampaignConfig,
) -> Result<(), ContractError> {
    CAMPAIGN_CONFIG
        .save(storage, &config)
        .map_err(ContractError::Std)
}

/// Only used on the instantiation
pub(crate) fn set_tiers(storage: &mut dyn Storage, tiers: Vec<Tier>) -> Result<(), ContractError> {
    for tier in tiers {
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

// pub(crate) fn validate_tiers(storage: &mut dyn Storage) -> bool {
//     !TIERS.is_empty(storage)
//         && TIERS
//             .range_raw(storage, None, None, Order::Ascending)
//             .any(|res| res.unwrap().1.limit.is_none())
// }

pub(crate) fn get_current_stage(storage: &dyn Storage) -> CampaignStage {
    CAMPAIGN_STAGE.load(storage).unwrap_or(CampaignStage::READY)
}
