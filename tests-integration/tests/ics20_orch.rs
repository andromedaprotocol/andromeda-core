#![cfg(not(target_arch = "wasm32"))]
use andromeda_fungible_tokens::ics20::ChannelResponse;
use andromeda_fungible_tokens::ics20::TransferMsg;
use andromeda_ics20::ICS20Contract;
use cosmwasm_std::Addr;
use cosmwasm_std::Uint128;
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;
use cw_orch_interchain::types::IbcPacketOutcome;
use cw_orch_interchain::InterchainEnv;

#[test]
fn test_ics20_ibc() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains").into_string();

    let interchain = MockInterchainEnv::new(vec![("juno", &sender), ("osmosis", &sender)]);

    let juno = interchain.chain("juno").unwrap();
    let osmosis = interchain.chain("osmosis").unwrap();

    juno.set_balance(sender.clone(), vec![Coin::new(100000000000000, "juno")])
        .unwrap();

    let ics20_juno = ICS20Contract::new(juno.clone());
    let ics20_osmosis = ICS20Contract::new(osmosis.clone());

    ics20_juno.upload().unwrap();
    ics20_osmosis.upload().unwrap();

    ics20_juno
        .instantiate(
            &andromeda_fungible_tokens::ics20::InitMsg {
                default_timeout: 120,
                gov_contract: "govcontract".to_string(),
                allowlist: vec![],
                default_gas_limit: None,
            },
            None,
            None,
        )
        .unwrap();

    ics20_osmosis
        .instantiate(
            &andromeda_fungible_tokens::ics20::InitMsg {
                default_timeout: 120,
                gov_contract: "govcontract".to_string(),
                allowlist: vec![],
                default_gas_limit: None,
            },
            None,
            None,
        )
        .unwrap();

    // Set up channel from juno to osmosis
    let channel_receipt = interchain
        .create_contract_channel(&ics20_juno, &ics20_osmosis, "ics20-1", None)
        .unwrap();

    // After channel creation is complete, we get the channel id, which is necessary for ICA remote execution
    let juno_channel = channel_receipt
        .interchain_channel
        .get_chain("juno")
        .unwrap()
        .channel
        .unwrap();

    let ics20_juno_transfer_request = ics20_juno
        .execute(
            &andromeda_fungible_tokens::ics20::ExecuteMsg::Transfer(TransferMsg {
                channel: juno_channel.to_string(),
                remote_address: ics20_osmosis.address().unwrap().into_string(),
                timeout: None,
                memo: None,
                message_recipient: None,
            }),
            Some(&[Coin::new(100_u128, "juno")]),
        )
        .unwrap();

    let packet_lifetime = interchain
        .wait_ibc("juno", ics20_juno_transfer_request)
        .unwrap();

    // For testing a successful outcome of the first packet sent out in the tx, you can use:
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0].outcome {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    } else {
        panic!("packet timed out");
        // There was a decode error or the packet timed out
        // Else the packet timed-out, you may have a relayer error or something is wrong in your application
    };
    let channel_response: ChannelResponse = ics20_juno
        .query(&andromeda_fungible_tokens::ics20::QueryMsg::Channel {
            id: juno_channel.to_string(),
        })
        .unwrap();
    let amount = channel_response.total_sent[0].amount();
    assert_eq!(amount, Uint128::new(100));
}
