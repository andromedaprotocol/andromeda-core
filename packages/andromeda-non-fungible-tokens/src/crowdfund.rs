use andromeda_std::amp::addresses::AndrAddr;
use andromeda_std::amp::Recipient;
use andromeda_std::common::denom::Asset;
use andromeda_std::common::expiration::Expiry;
use andromeda_std::common::{MillisecondsExpiration, OrderBy};
use andromeda_std::error::ContractError;
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, DepsMut, Env, Uint128, Uint64};
use cw20::Cw20ReceiveMsg;

use crate::cw721::TokenExtension;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    /// The configuration for the campaign
    pub campaign_config: CampaignConfig,
    /// The tiers for the campaign
    pub tiers: Vec<Tier>,
}

#[andr_exec]
#[cw_serde]
#[cfg_attr(not(target_arch = "wasm32"), derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    /// Add a tier
    #[attrs(restricted, nonpayable)]
    AddTier { tier: Tier },
    /// Update an existing tier
    #[attrs(restricted, nonpayable)]
    UpdateTier { tier: Tier },
    /// Remove a tier
    #[attrs(restricted, nonpayable)]
    RemoveTier { level: Uint64 },
    /// Start the campaign
    #[attrs(restricted)]
    StartCampaign {
        start_time: Option<Expiry>,
        end_time: Expiry,
        presale: Option<Vec<PresaleTierOrder>>,
    },
    /// Purchase tiers
    #[cfg_attr(not(target_arch = "wasm32"), cw_orch(payable))]
    PurchaseTiers { orders: Vec<SimpleTierOrder> },
    /// Purchase tiers with cw20
    #[attrs(nonpayable)]
    Receive(Cw20ReceiveMsg),
    /// End the campaign
    #[attrs(restricted, nonpayable)]
    EndCampaign {},
    /// Claim tiers or get refunded based on the campaign result
    Claim {},
    /// Discard the campaign
    #[attrs(restricted, nonpayable)]
    DiscardCampaign {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    PurchaseTiers { orders: Vec<SimpleTierOrder> },
}

#[cw_serde]
pub struct CampaignConfig {
    /// Title of the campaign. Maximum length is 64.
    pub title: Option<String>,
    /// Short description about the campaign.
    pub description: Option<String>,
    /// URL for the banner of the campaign
    pub banner: Option<String>,
    /// Official website of the campaign
    pub url: Option<String>,
    /// The address of the tier contract whose tokens are being distributed
    pub token_address: AndrAddr,
    /// The denom of the token that is being accepted by the campaign
    pub denom: Asset,
    /// Recipient that is upposed to receive the funds gained by the campaign
    pub withdrawal_recipient: Recipient,
    /// The minimum amount of funding to be sold for the successful fundraising
    pub soft_cap: Option<Uint128>,
    /// The maximum amount of funding to be sold for the fundraising
    pub hard_cap: Option<Uint128>,
}

impl CampaignConfig {
    pub fn validate(&self, deps: DepsMut, env: &Env) -> Result<(), ContractError> {
        // validate addresses
        self.token_address.validate(deps.api)?;
        self.withdrawal_recipient.validate(&deps.as_ref())?;
        let _ = self
            .denom
            .get_verified_asset(deps, env.clone())
            .map_err(|_| ContractError::InvalidAsset {
                asset: self.denom.to_string(),
            })?;

        // validate meta info
        ensure!(
            self.title.clone().unwrap_or_default().len() <= 64,
            ContractError::InvalidParameter {
                error: Some("Title length can be 64 at maximum".to_string())
            }
        );

        // validate target capital
        ensure!(
            (self.soft_cap).map_or(true, |soft_cap| soft_cap
                < self.hard_cap.unwrap_or(soft_cap + Uint128::new(1))),
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
    SUCCESS,
    /// Stage when campaign failed to meet the target cap before expiration
    FAILED,
    /// Stage when campaign is discarded
    DISCARDED,
}

impl std::fmt::Display for CampaignStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::READY => write!(f, "READY"),
            Self::ONGOING => write!(f, "ONGOING"),
            Self::SUCCESS => write!(f, "SUCCESS"),
            Self::FAILED => write!(f, "FAILED"),
            Self::DISCARDED => write!(f, "DISCARDED"),
        }
    }
}

#[cw_serde]
pub struct Tier {
    pub level: Uint64,
    pub label: String,
    pub price: Uint128,
    pub limit: Option<Uint128>, // None for no limit
    pub metadata: TierMetaData,
}

#[cw_serde]
pub struct TierOrder {
    pub orderer: Addr,
    pub level: Uint64,
    pub amount: Uint128,
    pub is_presale: bool,
}

// Used for presale
#[cw_serde]
pub struct PresaleTierOrder {
    pub level: Uint64,
    pub amount: Uint128,
    pub orderer: Addr,
}

impl From<PresaleTierOrder> for TierOrder {
    fn from(val: PresaleTierOrder) -> Self {
        TierOrder {
            level: val.level,
            amount: val.amount,
            orderer: val.orderer,
            is_presale: true,
        }
    }
}

// Used when the orderer is defined
#[cw_serde]
pub struct SimpleTierOrder {
    pub level: Uint64,
    pub amount: Uint128,
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
        ensure!(
            !self.label.is_empty() && self.label.len() <= 64,
            ContractError::InvalidTier {
                operation: "all".to_string(),
                msg: "Label should be no-empty and its length can be 64 at maximum".to_string()
            }
        );

        Ok(())
    }
}
#[cw_serde]
pub struct TierMetaData {
    /// Universal resource identifier for the tier
    /// Should point to a JSON file that conforms to the CW721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// Any custom extension used by this contract
    pub extension: TokenExtension,
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query to get summary about the campaign.
    #[returns(CampaignSummaryResponse)]
    CampaignSummary {},
    /// Query to get TierOrders for a specific orderer.
    ///
    /// - orderer: The address of the orderer.
    /// - start_after: Optional parameter to indicate the starting point for pagination, based on the level of the TierOrder.
    /// - limit: Optional parameter to limit the number of results.
    /// - order_by: Optional parameter to specify the ordering of the results.
    #[returns(TierOrdersResponse)]
    TierOrders {
        orderer: String,
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    },
    /// Query to get Tiers used for the campaign.
    ///
    /// - start_after: Optional parameter to indicate the starting point for pagination, based on the level of the Tier.
    /// - limit: Optional parameter to limit the number of results.
    /// - order_by: Optional parameter to specify the ordering of the results.
    #[returns(TiersResponse)]
    Tiers {
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    },
}

#[cw_serde]
pub struct CampaignSummaryResponse {
    // Campaign configuration
    pub title: Option<String>,
    pub description: Option<String>,
    pub banner: Option<String>,
    pub url: Option<String>,
    pub token_address: AndrAddr,
    pub denom: Asset,
    pub withdrawal_recipient: Recipient,
    pub soft_cap: Option<Uint128>,
    pub hard_cap: Option<Uint128>,
    pub start_time: Option<MillisecondsExpiration>,
    pub end_time: MillisecondsExpiration,
    // Current Status
    pub current_stage: String,
    pub current_capital: u128,
}

#[cw_serde]
pub struct TierOrdersResponse {
    pub orders: Vec<SimpleTierOrder>,
}

#[cw_serde]
pub struct TiersResponse {
    pub tiers: Vec<TierResponseItem>,
}

#[cw_serde]
pub struct TierResponseItem {
    pub tier: Tier,
    pub sold_amount: Uint128,
}
