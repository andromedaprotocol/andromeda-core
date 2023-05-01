use cosmwasm_std::{BankMsg, Coin, SubMsg};

pub mod mock_querier;

/// Generates a bank send message for quick generation
pub fn bank_sub_msg(recipient: impl Into<String>, amount: Vec<Coin>) -> SubMsg {
    SubMsg::new(BankMsg::Send {
        to_address: recipient.into(),
        amount,
    })
}
