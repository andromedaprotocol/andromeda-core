use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg};
use andromeda_modules::rates::{Rate, RateInfo};
use andromeda_non_fungible_tokens::cw721::{
    ExecuteMsg as CW721ExecuteMsg, TokenExtension, TransferAgreement,
};
use andromeda_rates::mock::{mock_andromeda_rates, mock_rates_instantiate_msg};
use andromeda_testing::mock::MockAndromeda;
use common::{
    ado_base::{modules::Module, recipient::Recipient},
    primitive::Value,
};
use cosmwasm_std::{coin, Addr, Uint128};
use cw721_base::MintMsg;
use cw_multi_test::{App, Executor};

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

fn mock_app() -> App {
    App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("owner"),
                [coin(999999, "uandr")].to_vec(),
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("buyer"),
                [coin(1000, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

#[test]
fn cw721_rates_module() {
    let mut router = mock_app();
    let owner = Addr::unchecked("owner");

    // Store contract codes
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let rates_code_id = router.store_code(mock_andromeda_rates());

    let receiver = Addr::unchecked("receiver");

    let andr = mock_andromeda(&mut router, owner.clone());

    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "rates", rates_code_id);

    // Generate rates contract
    let rates: Vec<RateInfo> = [RateInfo {
        rate: Rate::Flat(coin(100, "uandr")),
        is_additive: true,
        recipients: [Recipient::from_string(receiver.to_string())].to_vec(),
        description: Some("Some test rate".to_string()),
    }]
    .to_vec();
    let rates_init_msg = mock_rates_instantiate_msg(rates);
    let rates_addr = router
        .instantiate_contract(
            rates_code_id,
            owner.clone(),
            &rates_init_msg,
            &[],
            "rates",
            None,
        )
        .unwrap();

    // Generate CW721 contract
    let modules: Vec<Module> = [Module {
        module_name: Some("rates".to_string()),
        address: rates_addr.to_string(),

        is_mutable: false,
    }]
    .to_vec();
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        Some(andr.kernel.addr().to_string()),
    );
    let cw721_addr = router
        .instantiate_contract(
            cw721_code_id,
            owner.clone(),
            &cw721_init_msg,
            &[],
            "cw721",
            None,
        )
        .unwrap();

    // Mint Token
    let token_id = "1".to_string();
    let token_extension = TokenExtension {
        name: "test token".to_string(),
        publisher: owner.to_string(),
        description: None,
        attributes: Vec::new(),
        image: "".to_string(),
        image_data: None,
        external_url: None,
        animation_url: None,
        youtube_url: None,
    };
    let mint_msg = CW721ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: token_id.clone(),
        owner: owner.to_string(),
        token_uri: None,
        extension: token_extension,
    }));
    let _ = router
        .execute_contract(owner.clone(), cw721_addr.clone(), &mint_msg, &[])
        .unwrap();

    // Create Transfer Agreement
    let buyer = Addr::unchecked("buyer");
    let agreement_amount = coin(100, "uandr");
    let xfer_agreement_msg = CW721ExecuteMsg::TransferAgreement {
        token_id: token_id.clone(),
        agreement: Some(TransferAgreement {
            amount: Value::Raw(agreement_amount),
            purchaser: buyer.to_string(),
        }),
    };
    let _ = router
        .execute_contract(owner.clone(), cw721_addr.clone(), &xfer_agreement_msg, &[])
        .unwrap();

    // Store current balances for comparison
    let pre_balance_owner = router
        .wrap()
        .query_balance(owner.to_string(), "uandr")
        .unwrap();
    let pre_balance_receiver = router
        .wrap()
        .query_balance(receiver.to_string(), "uandr")
        .unwrap();

    // Transfer Token
    let xfer_msg = CW721ExecuteMsg::TransferNft {
        recipient: buyer.to_string(),
        token_id,
    };
    let _ = router
        .execute_contract(buyer, cw721_addr, &xfer_msg, &[coin(200, "uandr")])
        .unwrap();

    // Check balances post tx
    let post_balance_owner = router
        .wrap()
        .query_balance(owner.to_string(), "uandr")
        .unwrap();
    let post_balance_receiver = router
        .wrap()
        .query_balance(receiver.to_string(), "uandr")
        .unwrap();
    assert_eq!(
        pre_balance_owner.amount + Uint128::from(100u128),
        post_balance_owner.amount
    );
    assert_eq!(
        pre_balance_receiver.amount + Uint128::from(100u128),
        post_balance_receiver.amount
    );
}
