use andromeda_non_fungible_tokens::crowdfund::CampaignConfig;
use cw_storage_plus::Item;

pub const CAMPAIGN_CONFIG: Item<CampaignConfig> = Item::new("campaign_config");
