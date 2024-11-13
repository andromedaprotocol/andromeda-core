use andromeda_fungible_tokens::cw20_staking::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "cw20-staking";

contract_interface!(
    CW20StakingContract,
    CONTRACT_ID,
    "andromeda_cw20_staking.wasm"
);
