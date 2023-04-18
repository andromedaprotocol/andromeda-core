use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_get_components_msg,
};
use andromeda_automation::counter::ExecuteMsg as CounterExecuteMsg;
use andromeda_counter::mock::{mock_andromeda_counter, mock_counter_instantiate_msg};
use andromeda_kernel::mock::{mock_amp_direct, mock_get_key_address, mock_upsert_key_address};
use andromeda_message_bridge::mock::{
    mock_andromeda_message_bridge, mock_message_bridge_instantiate_msg, mock_save_channel,
    mock_send_amp_message,
};

use andromeda_os::messages::AMPMsg;
use andromeda_testing::mock::MockAndromeda;
use andromeda_vfs::mock::mock_resolve_path_query;
use cosmwasm_std::{coin, coins, to_binary, Addr};
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

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn kernel() {
    let owner = Addr::unchecked("owner");
    // let recipient = Addr::unchecked("recipient");
    // let recipient2 = Addr::unchecked("recipient2");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    let message_bridge_code_id = router.store_code(mock_andromeda_message_bridge());
    let counter_code_id = router.store_code(mock_andromeda_counter());
    let app_code_id = router.store_code(mock_andromeda_app());

    andr.store_code_id(&mut router, "message-bridge", message_bridge_code_id);
    andr.store_code_id(&mut router, "counter", counter_code_id);
    andr.store_code_id(&mut router, "app", app_code_id);

    // Generate Message Bridge Contract
    let message_bridge_init_msg =
        mock_message_bridge_instantiate_msg(Some(andr.kernel_address.to_string()));
    let messsage_bridge_app_component = AppComponent::new(
        "message-bridge",
        "message-bridge",
        to_binary(&message_bridge_init_msg).unwrap(),
    );

    // Generate Counter Contract

    let counter_init_msg = mock_counter_instantiate_msg(andr.kernel_address.to_string());
    let counter_app_component =
        AppComponent::new("counter", "counter", to_binary(&counter_init_msg).unwrap());

    let app_components: Vec<AppComponent> =
        vec![messsage_bridge_app_component, counter_app_component];

    let app_init_msg = mock_app_instantiate_msg(
        "app1",
        app_components.clone(),
        andr.kernel_address.to_string(),
    );

    let app_addr = router
        .instantiate_contract(
            app_code_id,
            owner.clone(),
            &app_init_msg,
            &[],
            "Crowdfund App",
            Some(owner.to_string()),
        )
        .unwrap();

    let components: Vec<AppComponent> = router
        .wrap()
        .query_wasm_smart(app_addr.clone(), &mock_get_components_msg())
        .unwrap();

    assert_eq!(components, app_components);

    let _counter_addr = andr.vfs_resolve_path(&mut router, "/am/app1/counter");
    let message_bridge_addr = andr.vfs_resolve_path(&mut router, "/am/app1/message-bridge");

    // Ensure hidden component is not added to VFS
    let vfs_address_query = mock_get_key_address("vfs");
    let vfs_address: Addr = router
        .wrap()
        .query_wasm_smart(andr.kernel_address.clone(), &vfs_address_query)
        .unwrap();

    let query = mock_resolve_path_query("/am/app1/.hidden_vault");
    assert!(router
        .wrap()
        .query_wasm_smart::<Addr>(vfs_address, &query)
        .is_err());

    // Save channel in message bridge
    let save_channel_msg = mock_save_channel("channel-1".to_string(), "juno".to_string());
    router
        .execute_contract(
            app_addr,
            message_bridge_addr.clone(),
            &save_channel_msg,
            &[],
        )
        .unwrap();

    // Upsert IBC Bridge address into kernel
    let upsert_msg = mock_upsert_key_address("ibc-bridge", message_bridge_addr.clone());
    router
        .execute_contract(
            owner.clone(),
            andr.kernel_address.clone(),
            &upsert_msg,
            &coins(10, "uandr"),
        )
        .unwrap();

    // Create a direct AMP message
    let recipient = "ibc://juno/user_1/app2/counter";
    let message = to_binary(&CounterExecuteMsg::IncrementOne {}).unwrap();
    let _send_msg = mock_amp_direct(recipient, message.clone(), None, None, None);
    let amp_msg = vec![AMPMsg::new(recipient, message, None, None, None, None)];
    let send_amp_pkt_msg = mock_send_amp_message("juno".to_string(), amp_msg);
    let _res = router
        .execute_contract(
            owner,
            message_bridge_addr,
            &send_amp_pkt_msg,
            &coins(100, "uandr"),
        )
        .unwrap();

    // // So far the kernel is successfully sending a packet to the relevant message bridge using the parser
    // let res = router
    //     .execute_contract(owner, andr.kernel_address, &send_msg, &coins(100, "uandr"))
    //     .unwrap();
    // println!("{:?}", res)

    // let query_balance =
    //     mock_vault_get_balance(recipient.to_string(), Some("uandr".to_string()), None);
    // let query_balance2 =
    //     mock_vault_get_balance(recipient2.to_string(), Some("uandr".to_string()), None);

    // let resp: Vec<Coin> = router
    //     .wrap()
    //     .query_wasm_smart(vault_addr.clone(), &query_balance)
    //     .unwrap();
    // let resp2: Vec<Coin> = router
    //     .wrap()
    //     .query_wasm_smart(vault_addr, &query_balance2)
    //     .unwrap();

    // assert!(resp.first().is_some());
    // assert_eq!(resp.first().unwrap().amount, Uint128::from(80u128));
    // assert_eq!(resp2.first().unwrap().amount, Uint128::from(20u128));
}
