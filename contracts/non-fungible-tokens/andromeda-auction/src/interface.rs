use andromeda_non_fungible_tokens::auction::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "auction";

contract_interface!(AuctionContract, CONTRACT_ID, "andromeda_auction.wasm");
