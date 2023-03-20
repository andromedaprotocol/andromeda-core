use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg};

use andromeda_message_bridge::mock::{
    mock_andromeda_message_bridge, mock_message_bridge_channel_id,
    mock_message_bridge_instantiate_msg, mock_message_bridge_supported_chains, mock_save_channel,
};
use andromeda_non_fungible_tokens::cw721::{ExecuteMsg as CW721ExecuteMsg, TokenExtension};
use cosmwasm_std::{coin, Addr};
use cw721_base::MintMsg;

use cw_multi_test::{App, Executor};
// use cw_pause_once::PauseError;

// fn message_bridge_contract() -> Box<dyn Contract<Empty>> {
//     let contract = ContractWrapper::new(
//         cw721_base::entry::execute,
//         cw721_base::entry::instantiate,
//         cw721_base::entry::query,
//     );
//     Box::new(contract)
// }

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
fn message_bridge() {
    let mut router = mock_app();
    let owner = Addr::unchecked("owner");

    // Store contract codes
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let message_bridge_code_id = router.store_code(mock_andromeda_message_bridge());

    let _receiver = Addr::unchecked("receiver");

    // Generate message bridge contract

    let message_bridge_init_msg = mock_message_bridge_instantiate_msg();
    let message_bridge_addr = router
        .instantiate_contract(
            message_bridge_code_id,
            owner.clone(),
            &message_bridge_init_msg,
            &[],
            "message-bridge",
            None,
        )
        .unwrap();

    // Generate CW721 contract
    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        owner.to_string(),
        None,
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
        token_id: token_id,
        owner: owner.to_string(),
        token_uri: None,
        extension: token_extension,
    }));
    let _ = router
        .execute_contract(owner.clone(), cw721_addr, &mint_msg, &[])
        .unwrap();

    // Save channel
    let channel = "channel-1".to_string();
    let chain = "juno".to_string();
    let save_channel_msg = mock_save_channel(channel, chain.clone());

    let _ = router
        .execute_contract(
            owner.clone(),
            message_bridge_addr.clone(),
            &save_channel_msg,
            &[],
        )
        .unwrap();
    // Query channel id

    let channel_query_msg = mock_message_bridge_channel_id(chain);

    let channel: String = router
        .wrap()
        .query_wasm_smart(message_bridge_addr.clone(), &channel_query_msg)
        .unwrap();
    assert_eq!("channel-1".to_string(), channel);

    // Save another channel
    let channel = "channel-2".to_string();
    let chain = "secret".to_string();
    let save_channel_msg = mock_save_channel(channel, chain);

    let _ = router
        .execute_contract(owner, message_bridge_addr.clone(), &save_channel_msg, &[])
        .unwrap();

    // Query supported chains
    let supported_chains_query_msg = mock_message_bridge_supported_chains();

    let supported_chains: Vec<String> = router
        .wrap()
        .query_wasm_smart(message_bridge_addr, &supported_chains_query_msg)
        .unwrap();
    assert_eq!(vec!["secret", "juno"], supported_chains);

    // // Send message

    // let recipient = "recipient".to_string();
    // let chain = "juno".to_string();
    // let message = to_binary(&mint_msg).unwrap();
    // let send_msg = mock_send_message(recipient, chain, message);

    // let _ = router
    //     .execute_contract(owner.clone(), message_bridge_addr.clone(), &send_msg, &[])
    //     .unwrap();

    // // Store current balances for comparison
    // let pre_balance_owner = router
    //     .wrap()
    //     .query_balance(owner.to_string(), "uandr")
    //     .unwrap();
    // let pre_balance_receiver = router
    //     .wrap()
    //     .query_balance(receiver.to_string(), "uandr")
    //     .unwrap();

    // // Transfer Token
    // let xfer_msg = CW721ExecuteMsg::TransferNft {
    //     recipient: buyer.to_string(),
    //     token_id,
    // };
    // let _ = router
    //     .execute_contract(buyer, cw721_addr, &xfer_msg, &[coin(200, "uandr")])
    //     .unwrap();

    // // Check balances post tx
    // let post_balance_owner = router
    //     .wrap()
    //     .query_balance(owner.to_string(), "uandr")
    //     .unwrap();
    // let post_balance_receiver = router
    //     .wrap()
    //     .query_balance(receiver.to_string(), "uandr")
    //     .unwrap();
    // assert_eq!(
    //     pre_balance_owner.amount + Uint128::from(100u128),
    //     post_balance_owner.amount
    // );
    // assert_eq!(
    //     pre_balance_receiver.amount + Uint128::from(100u128),
    //     post_balance_receiver.amount
    // );
}
