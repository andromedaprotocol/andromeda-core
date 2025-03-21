use andromeda_fungible_tokens::cw20_redeem::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "cw20-redeem";

contract_interface!(
    Cw20RedeemContract,
    CONTRACT_ID,
    "andromeda_cw20_redeem.wasm"
);
