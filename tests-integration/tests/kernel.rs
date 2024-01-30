use andromeda_app_contract::mock::mock_andromeda_app;
use andromeda_finance::splitter::AddressPercent;
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg, MockSplitter,
};
use andromeda_std::amp::{AndrAddr, Recipient};
use andromeda_testing::{
    mock::MockAndromeda,
    mock_contract::{MockADO, MockContract},
};

use cosmwasm_std::{coin, Addr, Decimal};

use cw_multi_test::App;

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
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn kernel() {
    let owner = Addr::unchecked("owner");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    andr.store_ado(&mut router, mock_andromeda_app(), "app");
    andr.store_ado(&mut router, mock_andromeda_splitter(), "splitter");

    let splitter_msg = mock_splitter_instantiate_msg(
        vec![AddressPercent::new(
            Recipient::from_string(owner.to_string()).with_ibc_recovery(owner.clone()),
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
            Some(AndrAddr::from_string("~/am")),
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
    let splitter_owner = splitter.query_owner(&router);

    assert_eq!(splitter_owner, owner.to_string());

    let res = andr
        .kernel
        .execute_send(
            &mut router,
            owner,
            splitter.addr(),
            mock_splitter_send_msg(),
            vec![coin(100, "uandr")],
            None,
        )
        .unwrap();

    assert!(res.data.is_none());
}
