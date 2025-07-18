pub mod aos;

use aos::InterchainAOS;
use cosmwasm_std::Coin;
use cw_orch::mock::{cw_multi_test::MockApiBech32, MockBase};
use cw_orch_interchain::{core::InterchainEnv, mock::MockBech32InterchainEnv, prelude::*};

use cw_orch::prelude::*;
pub const DEFAULT_SENDER: &str = "sender_for_all_chains";

/// The `InterchainChain` struct represents a chain in the interchain environment.
/// It contains a `MockBase` instance representing the chain, a `String` for the chain name, and an `InterchainAOS` instance.
pub struct InterchainChain {
    pub chain: MockBase<MockApiBech32>,
    pub chain_name: String,
    pub aos: InterchainAOS,
    pub denom: String,
    pub addresses: Vec<String>,
    pub chain_id: String,
}

impl InterchainChain {
    pub fn new(chain: MockBase<MockApiBech32>, chain_name: String, chain_id: String) -> Self {
        let aos = InterchainAOS::new(chain.clone(), chain_name.clone());
        let (denom, addresses) = match chain_name.as_str() {
            "juno" => (
                "ujuno".to_string(),
                vec![
                    "juno12lm0kfn2g3gn39ulzvqnadwksss5ez8rk8ghm0".to_string(),
                    "juno10dx5rcshf3fwpyw8jjrh5m25kv038xkqz2r2yp".to_string(),
                ],
            ),
            "osmosis" => (
                "uosmo".to_string(),
                vec![
                    "osmo1qzskhrca90qy2yjjxqzq4yajy842x7c50xq33d".to_string(),
                    "osmo1v9jxgu33ta047h6lxa803d0j3qqwq2p4k0ahvu".to_string(),
                ],
            ),
            "andromeda" => (
                "uandro".to_string(),
                vec![
                    "andr10dx5rcshf3fwpyw8jjrh5m25kv038xkqvngnls".to_string(),
                    "andr12lm0kfn2g3gn39ulzvqnadwksss5ez8rc7rwq7".to_string(),
                ],
            ),
            _ => ("utoken".to_string(), vec![]),
        };

        Self {
            chain,
            chain_name,
            aos,
            denom,
            addresses,
            chain_id,
        }
    }
}

/// The `InterchainTestEnv` struct represents an environment for testing interchain interactions.
/// It contains three chains, `juno`, `osmosis`, and `andromeda`, each represented by an `InterchainChain` struct.
/// The `sender` field holds the address of the default sender for all chains.
/// The `interchain` field holds an instance of `MockInterchainEnv` which simulates the interchain environment.
pub struct InterchainTestEnv {
    pub juno: InterchainChain,
    pub osmosis: InterchainChain,
    pub andromeda: InterchainChain,
    pub interchain: MockBech32InterchainEnv,
}

impl InterchainTestEnv {
    pub fn new() -> Self {
        let interchain = MockBech32InterchainEnv::new(vec![
            ("juno-1", "juno"),
            ("osmosis-1", "osmo"),
            ("andromeda-1", "andr"),
        ]);

        // Setup Juno Chain
        let juno_chain = interchain.get_chain("juno-1").unwrap();
        let juno = InterchainChain::new(juno_chain, "juno".to_string(), "juno-1".to_string());

        juno.chain
            .set_balance(
                &juno.chain.sender,
                vec![Coin::new(100000000000000u128, "ujuno")],
            )
            .unwrap();

        // Setup Osmosis Chain
        let osmosis_chain = interchain.get_chain("osmosis-1").unwrap();
        let osmosis = InterchainChain::new(
            osmosis_chain,
            "osmosis".to_string(),
            "osmosis-1".to_string(),
        );

        osmosis
            .chain
            .set_balance(
                &osmosis.chain.sender,
                vec![Coin::new(10000000000000u128, "uosmo")],
            )
            .unwrap();

        // Setup Andromeda Chain
        let andromeda_chain = interchain.get_chain("andromeda-1").unwrap();
        let andromeda = InterchainChain::new(
            andromeda_chain,
            "andromeda".to_string(),
            "andromeda-1".to_string(),
        );

        andromeda
            .chain
            .set_balance(
                &andromeda.chain.sender,
                vec![Coin::new(100000000000000u128, "uandr")],
            )
            .unwrap();

        let interchain_test_env = Self {
            juno,
            osmosis,
            andromeda,
            interchain,
        };

        let chains = &[
            &interchain_test_env.juno,
            &interchain_test_env.osmosis,
            &interchain_test_env.andromeda,
        ];

        for (index, chain) in chains.iter().enumerate() {
            let other_chains = chains[index + 1..].to_vec();

            // Create channels between the current chain and all other chains
            for other_chain in other_chains {
                interchain_test_env.create_aos_channel(chain, other_chain);
            }
        }

        interchain_test_env
    }

    pub fn set_balance(&self, chain: &str, address: String, amount: Vec<Coin>) {
        let chain = self.interchain.get_chain(chain).unwrap();
        chain.addr_make_with_balance(address, amount).unwrap();
    }

    // Creates a contract channel between two kernels on the provided chains
    pub fn create_aos_channel(&self, chain_one: &InterchainChain, chain_two: &InterchainChain) {
        if chain_one.chain_name == chain_two.chain_name {
            panic!("Chains must be different");
        }

        // Create the channel and hold the receipt
        let channel_receipt = self
            .interchain
            .create_contract_channel(
                &chain_one.aos.kernel,
                &chain_two.aos.kernel,
                "andr-kernel-1",
                None,
            )
            .unwrap();
        let transfer_channel_receipt = self
            .interchain
            .create_channel(
                &chain_one.chain_id,
                &chain_two.chain_id,
                &PortId::transfer(),
                &PortId::transfer(),
                "ics20-1",
                None,
            )
            .unwrap();

        // Get the channel from the receipt for chain one
        let direct_channel = channel_receipt
            .interchain_channel
            .get_chain(&chain_one.chain_id)
            .unwrap()
            .channel
            .unwrap();
        let transfer_channel = transfer_channel_receipt
            .interchain_channel
            .get_chain(&chain_one.chain_id)
            .unwrap()
            .channel
            .unwrap();

        // Assign the channel to the kernel on chain one
        chain_one.aos.assign_channels(
            transfer_channel.to_string(),
            direct_channel.to_string(),
            chain_two.chain_name.clone(),
            chain_two.aos.kernel.address().unwrap().into_string(),
        );

        // Get the channel from the receipt for chain two
        let direct_channel = channel_receipt
            .interchain_channel
            .get_chain(&chain_two.chain_id)
            .unwrap()
            .channel
            .unwrap();
        let transfer_channel = transfer_channel_receipt
            .interchain_channel
            .get_chain(&chain_two.chain_id)
            .unwrap()
            .channel
            .unwrap();

        // Assign the channel to the kernel on chain two
        chain_two.aos.assign_channels(
            transfer_channel.to_string(),
            direct_channel.to_string(),
            chain_one.chain_name.clone(),
            chain_one.aos.kernel.address().unwrap().into_string(),
        );
    }
}

impl Default for InterchainTestEnv {
    fn default() -> Self {
        Self::new()
    }
}

pub fn ensure_packet_success(packet_lifetime: NestedPacketsFlow<MockBase<MockApiBech32>>) {
    if let IbcPacketOutcome::Success { .. } = &packet_lifetime.packets[0] {
        // Packet has been successfully acknowledged and decoded, the transaction has gone through correctly
    } else {
        panic!("Packet failed when success was expected");
    }
}
