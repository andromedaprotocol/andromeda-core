
use andromeda_finance::splitter::{UpdatedADORecipient, UpdatedAddressPercent, UpdatedRecipient};



use andromeda_splitter::mock::{
    mock_andromeda_splitter, mock_splitter_instantiate_msg, mock_splitter_send_kernel_msg,
};
use andromeda_testing::mock::MockAndromeda;
use andromeda_vault::mock::{
    mock_andromeda_vault, mock_vault_deposit_msg, mock_vault_get_balance,
    mock_vault_instantiate_msg,
};
use common::{
    ado_base::{recipient::Recipient},
};
use cosmwasm_std::{coin, coins, to_binary, Addr, Coin, Decimal, Uint128};

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
    let recipient = Addr::unchecked("recipient");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());

    // Store contract codes
    let vault_code_id = router.store_code(mock_andromeda_vault());
    let splitter_code_id = router.store_code(mock_andromeda_splitter());

    andr.store_code_id(&mut router, "splitter", splitter_code_id);
    andr.store_code_id(&mut router, "vault", vault_code_id);

    // Generate Vault Contract
    let vault_init_msg = mock_vault_instantiate_msg();
    let vault_addr = router
        .instantiate_contract(
            vault_code_id,
            owner.clone(),
            &vault_init_msg,
            &[],
            "Vault",
            Some(owner.to_string()),
        )
        .unwrap();

    // Generate Splitter Contract
    let recipients: Vec<UpdatedAddressPercent> = vec![UpdatedAddressPercent {
        recipient: UpdatedRecipient::ADO(UpdatedADORecipient {
            address: vault_addr.to_string(),
            msg: Some(
                to_binary(&mock_vault_deposit_msg(
                    Some(Recipient::Addr(recipient.to_string())),
                    None,
                    None,
                ))
                .unwrap(),
            ),
        }),
        percent: Decimal::percent(100),
    }];
    let splitter_init_msg = mock_splitter_instantiate_msg(recipients, andr.kernel_address, None);
    let splitter_addr = router
        .instantiate_contract(
            splitter_code_id,
            owner.clone(),
            &splitter_init_msg,
            &[],
            "Splitter",
            Some(owner.to_string()),
        )
        .unwrap();

    let send_msg = mock_splitter_send_kernel_msg(None, None);
    router
        .execute_contract(
            owner,
            splitter_addr,
            &send_msg,
            &coins(100, "uandr"),
        )
        .unwrap();

    let query_balance =
        mock_vault_get_balance(recipient.to_string(), Some("uandr".to_string()), None);

    let resp: Vec<Coin> = router
        .wrap()
        .query_wasm_smart(vault_addr, &query_balance)
        .unwrap();

    assert!(resp.first().is_some());
    assert_eq!(resp.first().unwrap().amount, Uint128::from(100u128))
}
