use andromeda_finance::rate_limiting_withdrawals::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "rate-limiting-withdrawals";

contract_interface!(
    RateLimitingWithdrawalsContract,
    CONTRACT_ID,
    "andromeda_rate_limiting_withdrawals.wasm"
);
