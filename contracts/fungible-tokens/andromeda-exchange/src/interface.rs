use andromeda_fungible_tokens::exchange::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "exchange";

contract_interface!(Cw20ExchangeContract, CONTRACT_ID, "andromeda_exchange.wasm");
