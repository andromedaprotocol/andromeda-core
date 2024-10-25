pub mod aos;

use aos::InterchainAOS;
use cosmwasm_std::{Addr, Coin};
use cw_orch::mock::MockBase;
use cw_orch_interchain::{InterchainEnv, MockInterchainEnv};

pub const DEFAULT_SENDER: &str = "sender_for_all_chains";

/// The `InterchainChain` struct represents a chain in the interchain environment.
/// It contains a `MockBase` instance representing the chain, a `String` for the chain name, and an `InterchainAOS` instance.
pub struct InterchainChain {
    pub chain: MockBase,
    pub chain_name: String,
    pub aos: InterchainAOS,
}

impl InterchainChain {
    pub fn new(chain: MockBase, chain_name: String) -> Self {
        let aos = InterchainAOS::new(chain.clone(), chain_name.clone());
        Self {
            chain,
            chain_name,
            aos,
        }
    }
}

/// The `InterchainTestEnv` struct represents an environment for testing interchain interactions.
/// It contains two chains, `juno` and `osmosis`, each represented by an `InterchainChain` struct.
/// The `sender` field holds the address of the default sender for all chains.
/// The `interchain` field holds an instance of `MockInterchainEnv` which simulates the interchain environment.
pub struct InterchainTestEnv {
    pub juno: InterchainChain,
    pub osmosis: InterchainChain,
    pub andromeda: InterchainChain,
    pub sender: Addr,
    pub interchain: MockInterchainEnv,
}

impl InterchainTestEnv {
    pub fn new() -> Self {
        let sender = Addr::unchecked(DEFAULT_SENDER);
        let interchain = MockInterchainEnv::new(vec![
            ("juno", DEFAULT_SENDER),
            ("osmosis", DEFAULT_SENDER),
            ("andromeda", DEFAULT_SENDER),
        ]);

        // Setup Juno Chain
        let juno_chain = interchain.get_chain("juno").unwrap();
        let juno = InterchainChain::new(juno_chain, "juno".to_string());

        // Setup Osmosis Chain
        let osmosis_chain = interchain.get_chain("osmosis").unwrap();
        let osmosis = InterchainChain::new(osmosis_chain, "osmosis".to_string());

        // Setup Andromeda Chain
        let andromeda_chain = interchain.get_chain("andromeda").unwrap();
        let andromeda = InterchainChain::new(andromeda_chain, "andromeda".to_string());

        let interchain_test_env = Self {
            juno,
            osmosis,
            andromeda,
            sender: sender.clone(),
            interchain,
        };

        // Assign balances to default sender
        interchain_test_env.set_balance(
            "juno",
            sender.to_string(),
            vec![Coin::new(100000000000000, "juno")],
        );
        interchain_test_env.set_balance(
            "osmosis",
            sender.to_string(),
            vec![Coin::new(100000000000000, "osmosis")],
        );
        interchain_test_env.set_balance(
            "andromeda",
            sender.to_string(),
            vec![Coin::new(100000000000000, "andromeda")],
        );

        // Create channels between chains
        interchain_test_env
            .create_aos_channel(&interchain_test_env.juno, &interchain_test_env.osmosis);
        interchain_test_env
            .create_aos_channel(&interchain_test_env.juno, &interchain_test_env.andromeda);
        interchain_test_env
            .create_aos_channel(&interchain_test_env.osmosis, &interchain_test_env.andromeda);

        interchain_test_env
    }

    pub fn set_balance(&self, chain: &str, address: String, amount: Vec<Coin>) {
        let chain = self.interchain.get_chain(chain).unwrap();
        chain.set_balance(address, amount).unwrap();
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

        // Get the channel from the receipt for chain one
        let channel = channel_receipt
            .interchain_channel
            .get_chain(&chain_one.chain_name)
            .unwrap()
            .channel
            .unwrap();

        // Assign the channel to the kernel on chain one
        chain_one
            .aos
            .assign_channels(channel.to_string(), chain_two.chain_name.clone());

        // Get the channel from the receipt for chain two
        let channel = channel_receipt
            .interchain_channel
            .get_chain(&chain_two.chain_name)
            .unwrap()
            .channel
            .unwrap();

        // Assign the channel to the kernel on chain two
        chain_two
            .aos
            .assign_channels(channel.to_string(), chain_one.chain_name.clone());
    }
}

impl Default for InterchainTestEnv {
    fn default() -> Self {
        Self::new()
    }
}
