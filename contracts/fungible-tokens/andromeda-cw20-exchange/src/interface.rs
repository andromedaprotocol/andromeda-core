use andromeda_fungible_tokens::cw20_exchange::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "cw20-exchange";

contract_interface!(
    Cw20ExchangeContract,
    CONTRACT_ID,
    "andromeda_cw20_exchange.wasm"
);
