use andromeda_std::amp::addresses::AndrAddr;
use andromeda_std::common::denom::validate_denom;
use andromeda_std::error::ContractError;
use andromeda_std::{andr_exec, andr_instantiate, andr_instantiate_modules, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Deps, Uint128, Uint64};

use crate::cw721::TokenExtension;

#[andr_instantiate]
#[andr_instantiate_modules]
#[cw_serde]
pub struct InstantiateMsg {
    /// The configuration for the campaign
    pub campaign_config: CampaignConfig,
    /// The tiers for the campaign
    pub tiers: Vec<Tier>,
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Add a tier
    AddTier { tier: Tier },
    /// Update an existing tier
    UpdateTier { tier: Tier },
    /// Remove a tier
    RemoveTier { level: Uint64 },

    /// Start the campaign
    StartCampaign {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct CampaignConfig {
    /// Title of the campaign. Maximum length is 64.
    pub title: String,
    /// Short description about the campaign.
    pub description: String,
    /// URL for the banner of the campaign
    pub banner: String,
    /// Official website of the campaign
    pub url: String,
    /// Withdrawal address for the funds gained by the campaign
    pub tier_address: AndrAddr,
    /// The address of the tier contract whose tokens are being distributed
    pub denom: String,
    /// The minimum amount of funding to be sold for the successful fundraising
    pub withdrawal_address: AndrAddr,
    /// The address of the tier contract whose tokens are being distributed
    pub soft_cap: Option<Uint128>,
    /// The maximum amount of funding to be sold for the fundraising
    pub hard_cap: Uint128,
}

impl CampaignConfig {
    pub fn validate(&self, deps: Deps) -> Result<(), ContractError> {
        // validate addresses
        self.tier_address.validate(deps.api)?;
        self.withdrawal_address.validate(deps.api)?;
        validate_denom(deps, self.denom.clone())?;

        // validate meta info
        ensure!(
            self.title.len() <= 64,
            ContractError::InvalidParameter {
                error: Some("Title length can be 64 at maximum".to_string())
            }
        );

        // validate target capital
        ensure!(
            (self.soft_cap).map_or(true, |soft_cap| soft_cap < self.hard_cap),
            ContractError::InvalidParameter {
                error: Some("soft_cap can not exceed hard_cap".to_string())
            }
        );
        Ok(())
    }
}

#[cw_serde]
pub enum CampaignStage {
    /// Stage when all necessary environment is set to start campaign
    READY,
    /// Stage when campaign is being carried out
    ONGOING,
    /// Stage when campaign is finished successfully
    SUCCEED,
    /// Stage when campaign failed to meet the target cap before expiration
    FAILED,
}

impl ToString for CampaignStage {
    #[inline]
    fn to_string(&self) -> String {
        match self {
            Self::READY => "READY".to_string(),
            Self::ONGOING => "ONGOING".to_string(),
            Self::SUCCEED => "SUCCEED".to_string(),
            Self::FAILED => "FAILED".to_string(),
        }
    }
}

#[cw_serde]
pub struct Tier {
    pub level: Uint64,
    pub price: Uint128,
    pub limit: Option<Uint128>, // None for no limit
    pub meta_data: TierMetaData,
}

impl Tier {
    pub fn validate(&self) -> Result<(), ContractError> {
        ensure!(
            !self.price.is_zero(),
            ContractError::InvalidTier {
                operation: "all".to_string(),
                msg: "Price can not be zero".to_string()
            }
        );
        Ok(())
    }
}
#[cw_serde]
pub struct TierMetaData {
    /// The owner of the tier
    pub owner: Option<String>,
    /// Universal resource identifier for the tier
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// Any custom extension used by this contract
    pub extension: TokenExtension,
}
