use andromeda_finance::fixed_amount_splitter::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_std::{ado_base::MigrateMsg, contract_interface, deploy::ADOMetadata};

pub const CONTRACT_ID: &str = "fixed-amount-splitter";

contract_interface!(
    FixedAmountSplitterContract,
    CONTRACT_ID,
    "andromeda_fixed_amount_splitter.wasm"
);

/// Macro to create a fixed amount splitter instantiate message
///
/// # Arguments
/// * `$env` - The test environment (e.g., juno.aos)
/// * `$recipients` - A vector of (recipient, denom, amount) tuples
#[macro_export]
macro_rules! fixed_amount_splitter_instantiate {
    // Single recipient with default amount (100)
    ($env:expr, $recipient:expr, $denom:expr) => {
        fixed_amount_splitter_instantiate!($env, [($recipient, $denom, Uint128::new(100))])
    };

    // Single recipient with custom amount
    ($env:expr, $recipient:expr, $denom:expr, $amount:expr) => {
        fixed_amount_splitter_instantiate!($env, [($recipient, $denom, $amount)])
    };

    // Multiple recipients with array syntax
    ($env:expr, [$(($recipient:expr, $denom:expr, $amount:expr)),*]) => {
        &andromeda_finance::fixed_amount_splitter::InstantiateMsg {
            recipients: Some(vec![
                $(
                    andromeda_finance::fixed_amount_splitter::AddressAmount {
                        recipient: Recipient {
                            address: AndrAddr::from_string($recipient.clone()),
                            msg: None,
                            ibc_recovery_address: None,
                        },
                        coins: vec![Coin {
                            denom: $denom.to_string(),
                            amount: $amount,
                        }],
                    }
                ),*
            ]),
            default_recipient: None,
            lock_time: None,
            kernel_address: $env.kernel.address().unwrap().into_string(),
            owner: None,
        }
    };
}
