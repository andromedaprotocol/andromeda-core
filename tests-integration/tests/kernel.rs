use andromeda_adodb::mock::mock_unpublish;
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

use cw_multi_test::{App, Executor};

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
    let user1 = "user1";

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());
    let splitter_code_id = router.store_code(mock_andromeda_splitter());

    let splitter_msg = mock_splitter_instantiate_msg(
        vec![AddressPercent::new(
            Recipient::from_string(user1.to_string()).with_ibc_recovery(owner.clone()),
            Decimal::one(),
        )],
        andr.kernel_address.clone(),
        None,
        None,
    );
    let splitter_addr = router
        .instantiate_contract(
            splitter_code_id,
            Addr::unchecked(user1),
            &splitter_msg,
            &[],
            "Splitter",
            None,
        )
        .unwrap();
    let kernel: MockContract = MockContract::from(andr.kernel_address.to_string());

    // This will return an error because this splitter contract's code id isn't part of the ADODB
    // It errors at the Kernel's AMPReceive Msg when it tries to verify the splitter's address
    assert!(kernel
        .execute(
            &mut router,
            KernelExecuteMsg::Send {
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
