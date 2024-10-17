use andromeda_std::error::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Coin, IbcEndpoint, Uint128};
use cw20::{Cw20Coin, Cw20ReceiveMsg};

#[cw_serde]
pub struct InitMsg {
    /// Default timeout for ics20 packets, specified in seconds
    pub default_timeout: u64,
    /// who can allow more contracts
    pub gov_contract: String,
    /// initial allowlist - all cw20 tokens we will send must be previously allowed by governance
    pub allowlist: Vec<AllowMsg>,
    /// If set, contracts off the allowlist will run with this gas limit.
    /// If unset, will refuse to accept any contract off the allow list.
    pub default_gas_limit: Option<u64>,
}

#[cw_serde]
pub struct AllowMsg {
    pub contract: String,
    pub gas_limit: Option<u64>,
}

#[cw_serde]
pub struct MigrateMsg {
    pub default_gas_limit: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
    /// This allows us to transfer *exactly one* native token
    Transfer(TransferMsg),
    /// This must be called by gov_contract, will allow a new cw20 token to be sent
    Allow(AllowMsg),
    /// Change the admin (must be called by current admin)
    UpdateAdmin { admin: String },
}

/// This is the message we accept via Receive
#[cw_serde]
pub struct TransferMsg {
    /// The local channel to send the packets on
    pub channel: String,
    /// The remote address to send to.
    /// Don't use HumanAddress as this will likely have a different Bech32 prefix than we use
    /// and cannot be validated locally
    pub remote_address: String,
    /// How long the packet lives in seconds. If not specified, use default_timeout
    pub timeout: Option<u64>,
    /// An optional memo to add to the IBC transfer
    pub memo: Option<String>,
    pub message_recipient: Option<MessageRecipient>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Return the port ID bound by this contract.
    #[returns(PortResponse)]
    Port {},
    /// Show all channels we have connected to.
    #[returns(ListChannelsResponse)]
    ListChannels {},
    /// Returns the details of the name channel, error if not created.
    #[returns(ChannelResponse)]
    Channel { id: String },
    /// Show the Config.
    #[returns(ConfigResponse)]
    Config {},
    #[returns(cw_controllers::AdminResponse)]
    Admin {},
    /// Query if a given cw20 contract is allowed.
    #[returns(AllowedResponse)]
    Allowed { contract: String },
    /// List all allowed cw20 contracts.
    #[returns(ListAllowedResponse)]
    ListAllowed {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ListChannelsResponse {
    pub channels: Vec<ChannelInfo>,
}

#[cw_serde]
pub struct ChannelResponse {
    /// Information on the channel's connection
    pub info: ChannelInfo,
    /// How many tokens we currently have pending over this channel
    pub balances: Vec<Amount>,
    /// The total number of tokens that have been sent over this channel
    /// (even if many have been returned, so balance is low)
    pub total_sent: Vec<Amount>,
}

#[cw_serde]
pub struct PortResponse {
    pub port_id: String,
}

#[cw_serde]
pub struct ConfigResponse {
    pub default_timeout: u64,
    pub default_gas_limit: Option<u64>,
    pub gov_contract: String,
}

#[cw_serde]
pub struct AllowedResponse {
    pub is_allowed: bool,
    pub gas_limit: Option<u64>,
}

#[cw_serde]
pub struct ListAllowedResponse {
    pub allow: Vec<AllowedInfo>,
}

#[cw_serde]
pub struct AllowedInfo {
    pub contract: String,
    pub gas_limit: Option<u64>,
}

#[cw_serde]
pub struct ChannelInfo {
    /// id of this channel
    pub id: String,
    /// the remote channel/port we connect to
    pub counterparty_endpoint: IbcEndpoint,
    /// the connection this exists on (you can use to query client/consensus info)
    pub connection_id: String,
}

#[cw_serde]
#[derive(Eq)]
pub struct MessageRecipient {
    pub message: Binary,
    pub recipient: String,
}

#[cw_serde]
pub enum Amount {
    Native(Coin),
    // FIXME? USe Cw20CoinVerified, and validate cw20 addresses
    Cw20(Cw20Coin),
}

impl Amount {
    // TODO: write test for this
    pub fn from_parts(denom: String, amount: Uint128) -> Self {
        if denom.starts_with("cw20:") {
            let address = denom.get(5..).unwrap().into();
            Amount::Cw20(Cw20Coin { address, amount })
        } else {
            Amount::Native(Coin { denom, amount })
        }
    }

    pub fn cw20(amount: u128, addr: &str) -> Self {
        Amount::Cw20(Cw20Coin {
            address: addr.into(),
            amount: Uint128::new(amount),
        })
    }

    pub fn native(amount: u128, denom: &str) -> Self {
        Amount::Native(Coin {
            denom: denom.to_string(),
            amount: Uint128::new(amount),
        })
    }
}

impl Amount {
    pub fn denom(&self) -> String {
        match self {
            Amount::Native(c) => c.denom.clone(),
            Amount::Cw20(c) => format!("cw20:{}", c.address.as_str()),
        }
    }

    pub fn amount(&self) -> Uint128 {
        match self {
            Amount::Native(c) => c.amount,
            Amount::Cw20(c) => c.amount,
        }
    }

    /// convert the amount into u64
    pub fn u64_amount(&self) -> Result<u64, ContractError> {
        Ok(self.amount().u128().try_into().unwrap())
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Amount::Native(c) => c.amount.is_zero(),
            Amount::Cw20(c) => c.amount.is_zero(),
        }
    }
}
