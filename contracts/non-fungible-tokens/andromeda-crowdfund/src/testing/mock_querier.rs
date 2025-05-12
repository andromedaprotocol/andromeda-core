use andromeda_non_fungible_tokens::{
    crowdfund::{CampaignConfig, Tier, TierMetaData},
    cw721::TokenExtension,
};
use andromeda_std::{
    ado_base::InstantiateMsg,
    ado_contract::ADOContract,
    amp::{AndrAddr, Recipient},
    common::denom::Asset,
    testing::mock_querier::{WasmMockQuerier, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage},
    Coin, OwnedDeps, QuerierWrapper, Uint128, Uint64,
};

pub const MOCK_DEFAULT_OWNER: &str =
    "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";
pub const RANDOM_USER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053x";
pub const ORDERER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053z";
pub const MOCK_TIER_CONTRACT: &str =
    "cosmwasm1jnurcdh67h0xwma5pfps9k9xzrl0gs4yjglkf4z66sc6z6f94frqp5kmk2";
pub const MOCK_WITHDRAWAL_ADDRESS: &str =
    "cosmwasm1dkt8wpsymxpna9gktg23henclzgs8vkv4tjcqm0h7qv32kzu0d7sw8kt6u";
pub const MOCK_DEFAULT_LIMIT: u128 = 100000;

pub fn mock_campaign_config(denom: Asset) -> CampaignConfig {
    CampaignConfig {
        title: Some("First Crowdfund".to_string()),
        description: Some("Demo campaign for testing".to_string()),
        banner: Some("http://<campaign_banner>".to_string()),
        url: Some("http://<campaign_url>".to_string()),
        denom,
        token_address: AndrAddr::from_string(MOCK_TIER_CONTRACT.to_owned()),
        withdrawal_recipient: Recipient::from_string(MOCK_WITHDRAWAL_ADDRESS.to_owned()),
        soft_cap: None,
        hard_cap: None,
    }
}

pub fn mock_campaign_tiers() -> Vec<Tier> {
    vec![
        Tier {
            level: Uint64::zero(),
            label: "Basic Tier".to_string(),
            limit: None,
            price: Uint128::new(10u128),
            metadata: TierMetaData {
                extension: TokenExtension {},
                token_uri: None,
            },
        },
        Tier {
            level: Uint64::new(1u64),
            label: "Tier 1".to_string(),
            limit: Some(Uint128::new(MOCK_DEFAULT_LIMIT)),
            price: Uint128::new(10u128),
            metadata: TierMetaData {
                extension: TokenExtension {},
                token_uri: None,
            },
        },
    ]
}

pub fn mock_zero_price_tier(level: Uint64) -> Tier {
    Tier {
        level,
        label: "Invalid Tier".to_string(),
        limit: None,
        price: Uint128::zero(),
        metadata: TierMetaData {
            extension: TokenExtension {},
            token_uri: None,
        },
    }
}

/// Alternative to `cosmwasm_std::testing::mock_dependencies` that allows us to respond to custom queries.
///
/// Automatically assigns a kernel address as MOCK_KERNEL_CONTRACT.
pub fn mock_dependencies_custom(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_TIER_CONTRACT, contract_balance)]));
    let storage = MockStorage::default();
    let mut deps = OwnedDeps {
        storage,
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: std::marker::PhantomData,
    };
    let sender = deps.api.addr_make("sender");
    ADOContract::default()
        .instantiate(
            &mut deps.storage,
            mock_env(),
            &deps.api,
            &QuerierWrapper::new(&deps.querier),
            message_info(&sender, &[]),
            InstantiateMsg {
                ado_type: "crowdfund".to_string(),
                ado_version: "test".to_string(),
                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();
    deps
}
