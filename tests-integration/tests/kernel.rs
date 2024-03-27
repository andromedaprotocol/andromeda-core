use andromeda_app_contract::mock::mock_andromeda_app;
use andromeda_finance::splitter::AddressPercent;
use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_msg, MockSplitter,
};
use andromeda_std::amp::{messages::AMPMsg, AndrAddr, Recipient};
use andromeda_testing::{
    mock::MockAndromeda,
    mock_contract::{MockADO, MockContract},
};
use andromeda_testing::{
    mock::{mock_app, MockAndromeda, MockApp},
    mock_contract::MockContract,
};

use cosmwasm_std::{coin, to_json_binary, Addr, Decimal};

use cw_multi_test::Executor;

// fn mock_app() -> App {
//     App::new(|router, _api, storage| {
//         router
//             .bank
//             .init_balance(
//                 storage,
//                 &Addr::unchecked("owner"),
//                 [coin(999999, "uandr")].to_vec(),
//             )
//             .unwrap();
//         router
//             .bank
//             .init_balance(
//                 storage,
//                 &Addr::unchecked("buyer"),
//                 [coin(1000, "uandr")].to_vec(),
//             )
//             .unwrap();
//         router
//             .bank
//             .init_balance(
//                 storage,
//                 &Addr::unchecked("user1"),
//                 [coin(1, "uandr")].to_vec(),
//             )
//             .unwrap();
//     })
// }

fn mock_andromeda(app: &mut MockApp, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn kernel() {
    let mut router = mock_app();
    let owner = router.api().addr_make("owner");
    let user1 = router.api().addr_make("user1");

    router
        .send_tokens(
            Addr::unchecked("owner"),
            owner.clone(),
            &[coin(1000, "uandr")],
        )
        .unwrap();
    let andr = mock_andromeda(&mut router, owner.clone());
    let splitter_code_id = router.store_code(mock_andromeda_splitter());

    let splitter_msg = mock_splitter_instantiate_msg(
        vec![AddressPercent::new(
            Recipient::from_string(user1.to_string()).with_ibc_recovery(owner.clone()),
            Decimal::one(),
        )],
        andr.kernel.addr().clone(),
        None,
        None,
    );
    let splitter_addr = router
        .instantiate_contract(
            splitter_code_id,
            user1.clone(),
            &splitter_msg,
            &[],
            "Splitter",
            None,
        )
        .unwrap();

    // This will return an error because this splitter contract's code id isn't part of the ADODB
    // It errors at the Kernel's AMPReceive Msg when it tries to verify the splitter's address
    assert!(andr
        .kernel
        .execute(
            &mut router,
            &andromeda_std::os::kernel::ExecuteMsg::Send {
                message: AMPMsg::new(
                    splitter_addr,
                    to_json_binary(&mock_splitter_send_msg()).unwrap(),
                    Some(vec![coin(100, "uandr")]),
                ),
            },
            owner.clone(),
            &[coin(100, "uandr")],
        )
        .is_err());

    // Works

    // Store contract codes
    andr.store_ado(&mut router, mock_andromeda_app(), "app");
    andr.store_ado(&mut router, mock_andromeda_splitter(), "splitter");

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
            owner,
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
        .query_balance(owner.clone(), "uandr".to_string())
        .unwrap();

    // The owner's balance should be his starting balance subtracted by the 100 he sent with the splitter execute msg
    assert_eq!(owner_balance, coin(900, "uandr"));

    assert!(res.data.is_none());
}
