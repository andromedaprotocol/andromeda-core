use cosmwasm_std::StdError;
use std::convert::From;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("InvalidPrimitive")]
    InvalidPrimitive {},

    #[error("IllegalTokenName")]
    IllegalTokenName {},

    #[error("IllegalTokenSymbol")]
    IllegalTokenSymbol {},

    #[error("MinterBlacklisted")]
    MinterBlacklisted {},

    #[error("EmptyRecipientsList")]
    EmptyRecipientsList {},

    #[error("AmountExceededHundredPrecent")]
    AmountExceededHundredPrecent {},

    #[error("InvalidAddress")]
    InvalidAddress {},

    #[error("EmptyFunds")]
    EmptyFunds {},

    #[error("ExpirationInPast")]
    ExpirationInPast {},

    #[error("ExpirationNotSpecified")]
    ExpirationNotSpecified {},

    #[error("CannotOverwriteHeldFunds")]
    CannotOverwriteHeldFunds {},

    #[error("ContractAddressNotInAddressList")]
    ContractAddressNotInAddressList {},

    #[error("ModuleNotUnique")]
    ModuleNotUnique {},

    #[error("InvalidRate")]
    InvalidRate {},

    #[error("InsufficientFunds")]
    InsufficientFunds {},

    #[error("NoPendingPayments")]
    NoPendingPayments {},

    #[error("NoReceivingAddress")]
    NoReceivingAddress {},

    #[error("ModuleDiscriptionTooLong")]
    ModuleDiscriptionTooLong { msg: String },

    #[error("SymbolInUse")]
    SymbolInUse {},

    #[error("ExceedsMaxAllowedCoins")]
    ExceedsMaxAllowedCoins {},

    #[error("NoLockedFunds")]
    NoLockedFunds {},

    #[error("FundsAreLocked")]
    FundsAreLocked {},

    #[error("InvalidTokenNameLength")]
    InvalidTokenNameLength { msg: String },

    #[error("TokenIsArchived")]
    TokenIsArchived {},

    #[error("InvalidMirrorFunds")]
    InvalidMirrorFunds { msg: String },

    #[error("InvalidJSON")]
    InvalidJSON {},

    #[error("ParsingError")]
    ParsingError { err: String },

    #[error("InvalidJSONField")]
    InvalidJSONField { key: String, expected: String },

    #[error("MissingJSON")]
    MissingJSON {},

    #[error("NestedAndromedaMsg")]
    NestedAndromedaMsg {},
}

impl From<FromUtf8Error> for ContractError {
    fn from(err: FromUtf8Error) -> Self {
        ContractError::Std(StdError::from(err))
    }
}
