use andromeda_fungible_tokens::airdrop::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "merkle_airdrop";

contract_interface!(
    MerkleAirdropContract,
    CONTRACT_ID,
    "andromeda_merkle_airdrop.wasm"
);
