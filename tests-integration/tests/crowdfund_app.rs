use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_crowdfund::mock::{
    mock_andromeda_crowdfund, mock_crowdfund_instantiate_msg, MockCrowdfund,
};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg};
use andromeda_non_fungible_tokens::{crowdfund::{CampaignConfig, CampaignStage, TierMetaData}, cw721::TokenExtension};
use andromeda_std::{
    amp::{AndrAddr, Recipient},
    common::{denom::Asset, MillisecondsExpiration},
};
use andromeda_testing::{mock::mock_app, mock_builder::MockAndromedaBuilder, MockContract};
use cosmwasm_std::{coin, to_json_binary, Uint64, Uint128};

#[test]
fn test_crowdfund_app_native() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![]),
            ("buyer_one", vec![coin(1000, "uandr")]),
            ("recipient", vec![]),
        ])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("crowdfund", mock_andromeda_crowdfund()),
            ("app-contract", mock_andromeda_app()),
        ])
        .build(&mut router);

    let owner = andr.get_wallet("owner");
    // let buyer_one = andr.get_wallet("buyer_one");
    let recipient = Recipient::new(andr.get_wallet("recipient"), None);

    // Generate App Components
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Campaign Tier".to_string(),
        "CT".to_string(),
        owner.to_string(),
        None,
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        "cw721".to_string(),
        "cw721".to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let campaign_config = mock_campaign_config(
        Asset::NativeToken("uandr".to_string()),
        AndrAddr::from_string(format!("./{}", cw721_component.name)),
        recipient,
    );
    let crowdfund_init_msg = mock_crowdfund_instantiate_msg(
        campaign_config,
        vec![],
        None,
        andr.kernel.addr(),
        Some(owner.to_string()),
    );
    let crowdfund_component = AppComponent::new(
        "crowdfund".to_string(),
        "crowdfund".to_string(),
        to_json_binary(&crowdfund_init_msg).unwrap(),
    );

    let app_components = vec![cw721_component.clone(), crowdfund_component.clone()];

    let app = MockAppContract::instantiate(
        andr.get_code_id(&mut router, "app-contract"),
        owner,
        &mut router,
        "Crowdfund App",
        app_components.clone(),
        andr.kernel.addr(),
        Some(owner.to_string()),
    );

    let components = app.query_components(&router);
    assert_eq!(components, app_components);

    let crowdfund: MockCrowdfund =
        app.query_ado_by_component_name(&router, crowdfund_component.name);

    let summary = crowdfund.query_campaign_summary(&mut router);
    assert_eq!(summary.current_cap, 0);
    assert_eq!(summary.current_stage, CampaignStage::READY.to_string());

    // Add tiers
    let meta_data= TierMetaData {
        token_uri: None,
        extension: TokenExtension {
            ..Default::default()
        }
    };
    crowdfund.execute_add_tier(owner.clone(), &mut router, Uint64::one(), "Tier 1".to_string(), Uint128::new(100), None, meta_data).unwrap();
}

fn mock_campaign_config(
    denom: Asset,
    token_address: AndrAddr,
    withdrawal_recipient: Recipient,
) -> CampaignConfig {
    CampaignConfig {
        title: "First Crowdfund".to_string(),
        description: "Demo campaign for testing".to_string(),
        banner: "http://<campaign_banner>".to_string(),
        url: "http://<campaign_url>".to_string(),
        denom,
        token_address: token_address,
        withdrawal_recipient,
        soft_cap: None,
        hard_cap: None,
        start_time: None,
        end_time: MillisecondsExpiration::zero(),
    }
}
