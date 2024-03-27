use andromeda_finance::splitter::AddressPercent;
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg, MockSplitter,
};
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_testing::{
    mock::mock_app,
    mock_builder::MockAndromedaBuilder,
    mock_contract::{MockADO, MockContract},
};

use cosmwasm_std::{coin, Addr, Decimal};

#[test]
fn kernel() {
    let mut router = mock_app(None);
    let andr = MockAndromedaBuilder::new(&mut router, "admin")
        .with_wallets(vec![
            ("owner", vec![coin(1000, "uandr")]),
            ("user1", vec![]),
        ])
        .with_contracts(vec![("splitter", mock_andromeda_splitter())])
        .build(&mut router);

    let owner = andr.get_wallet("owner");
    let user1 = andr.get_wallet("user1");

    let splitter_msg = mock_splitter_instantiate_msg(
        vec![AddressPercent::new(
            Recipient::from_string(user1.to_string()).with_ibc_recovery(owner.clone()),
            Decimal::one(),
        )],
        andr.kernel.addr().clone(),
        None,
        None,
    );

    let res = andr
        .kernel
        .execute_create(
            &mut router,
            owner.clone(),
            "splitter",
            splitter_msg,
            Some(AndrAddr::from_string(andr.admin_address.to_string())),
            None,
        )
        .unwrap();

    let event_key = res
        .events
        .iter()
        .position(|ev| ev.ty == "instantiate")
        .unwrap();
    let inst_event = res.events.get(event_key).unwrap();
    let attr_key = inst_event
        .attributes
        .iter()
        .position(|attr| attr.key == "_contract_address")
        .unwrap();
    let attr = inst_event.attributes.get(attr_key).unwrap();
    let addr: Addr = Addr::unchecked(attr.value.clone());
    let splitter = MockSplitter::from(addr);
    splitter
        .accept_ownership(&mut router, andr.admin_address.clone())
        .unwrap();

    let splitter_owner = splitter.query_owner(&router);

    assert_eq!(splitter_owner, andr.admin_address.to_string());

    let res = andr
        .kernel
        .execute_send(
            &mut router,
            owner.clone(),
            splitter.addr(),
            mock_splitter_send_msg(),
            vec![coin(100, "uandr")],
            None,
        )
        .unwrap();

    let user1_balance = router
        .wrap()
        .query_balance(user1, "uandr".to_string())
        .unwrap();

    // user1 had one coin before the splitter execute msg which is expected to increase his balance by 100uandr
    assert_eq!(user1_balance, coin(100, "uandr"));

    let owner_balance = router
        .wrap()
        .query_balance(owner, "uandr".to_string())
        .unwrap();

    // The owner's balance should be his starting balance subtracted by the 100 he sent with the splitter execute msg
    assert_eq!(owner_balance, coin(900, "uandr"));

    assert!(res.data.is_none());
}
