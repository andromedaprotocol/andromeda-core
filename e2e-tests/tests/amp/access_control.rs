use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{mock_andromeda_app, MockAppContract};
use andromeda_cw721::mock::{
    mock_andromeda_cw721, mock_cw721_instantiate_msg, mock_quick_mint_msg, MockCW721,
};
use andromeda_non_fungible_tokens::cw721::ExecuteMsg;
use andromeda_std::{
    ado_base::permissioning::{LocalPermission, Permission, PermissioningMessage},
    amp::AndrAddr,
};
use andromeda_testing::{
    mock::mock_app, mock_builder::MockAndromedaBuilder, MockAndromeda, MockContract,
};
use cosmwasm_std::{coin, to_json_binary};
use cw_multi_test::{App, BankKeeper, MockApiBech32};
use rstest::*;

const FALSE_USER: &str = "false_user";

const CW721_OWNER: &str = "owner";
const CW721_USER: &str = "user1";
const CW721_APP_NAME: &str = "app";
const CW721_COMPONENT_NAME: &str = "cw721";
const CW721_MINT_ACTION: &str = "Mint";
#[fixture]
fn setup_cw721() -> (App<BankKeeper, MockApiBech32>, MockAndromeda, MockCW721) {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            (CW721_OWNER, vec![coin(1000, "uandr")]),
            (CW721_USER, vec![]),
            (FALSE_USER, vec![]),
        ])
        .with_contracts(vec![
            ("cw721", mock_andromeda_cw721()),
            ("app-contract", mock_andromeda_app()),
        ])
        .build(&mut router);

    let owner = andr.get_wallet(CW721_OWNER);
    let user = andr.get_wallet(CW721_USER);
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        andr.kernel.addr().to_string(),
        None,
    );
    let cw721_component = AppComponent::new(
        CW721_COMPONENT_NAME.to_string(),
        CW721_COMPONENT_NAME.to_string(),
        to_json_binary(&cw721_init_msg).unwrap(),
    );

    let app_code_id = andr.get_code_id(&mut router, "app-contract");
    let app = MockAppContract::instantiate(
        app_code_id,
        owner,
        &mut router,
        CW721_APP_NAME.to_string(),
        vec![cw721_component.clone()],
        andr.kernel.addr(),
        None,
    );

    let components = app.query_components(&router);
    assert_eq!(components, vec![cw721_component]);

    let cw721 = app.query_ado_by_component_name::<MockCW721>(&router, CW721_COMPONENT_NAME);

    let permission_action_msg = ExecuteMsg::Permissioning(PermissioningMessage::PermissionAction {
        action: CW721_MINT_ACTION.to_string(),
    });
    cw721
        .execute(&mut router, &permission_action_msg, owner.clone(), &[])
        .unwrap();

    let permission_msg = ExecuteMsg::Permissioning(PermissioningMessage::SetPermission {
        actors: vec![AndrAddr::from_string(user)],
        action: CW721_MINT_ACTION.to_string(),
        permission: Permission::Local(LocalPermission::whitelisted(None)),
    });

    cw721
        .execute(&mut router, &permission_msg, owner.clone(), &[])
        .unwrap();

    (router, andr, cw721)
}

#[rstest]
#[case::owner(CW721_OWNER, true)]
#[case::user(CW721_USER, true)]
#[case::false_user(FALSE_USER, false)]
fn test_mint_permission(
    setup_cw721: (App<BankKeeper, MockApiBech32>, MockAndromeda, MockCW721),
    #[case] sender: &str,
    #[case] expected_success: bool,
) {
    let (mut router, andr, cw721) = setup_cw721;

    let mint_msg = mock_quick_mint_msg(1, andr.get_wallet(sender).to_string());
    let res = andr.kernel.execute_send(
        &mut router,
        andr.get_wallet(sender).clone(),
        cw721.addr().to_string(),
        mint_msg,
        vec![],
        None,
    );

    assert_eq!(res.is_ok(), expected_success);

    if expected_success {
        let owner = cw721.query_owner_of(&router, "0");
        assert_eq!(owner, andr.get_wallet(sender).to_string());
    }
}
