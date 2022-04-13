use cosmwasm_std::StdError;
use cw0::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Didn't send any funds")]
    NoFunds {},
    #[error("Only supports channel with ibc version ics20-1, got {version}")]
    InvalidIbcVersion { version: String },
    #[error("Only supports unordered channel")]
    OnlyOrderedChannel {},
    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },
    #[error("Channel doesn't exist: {id}")]
    NoSuchChannel { id: String },
    #[error("You can only send cw20 tokens that have been explicitly allowed by governance")]
    NotOnAllowList,
    #[error("Amount larger than 2**64, not supported by ics20 packets")]
    AmountOverflow {},
    #[error("Insufficient funds to redeem voucher on channel")]
    InsufficientFunds {},
    #[error("Only accepts tokens that originate on this chain, not native tokens of remote chain")]
    NoForeignTokens {},
    #[error("Parsed port from denom ({port}) doesn't match packet")]
    FromOtherPort { port: String },
    #[error("Parsed channel from denom ({channel}) doesn't match packet")]
    FromOtherChannel { channel: String },
}

/// Never is a placeholder to ensure we don't return any errors
#[derive(Error, Debug)]
pub enum Never {}
