use andromeda_std::{
    amp::AndrAddr, andr_exec, andr_instantiate, andr_instantiate_modules, andr_query,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Uint128};
use cw20::{Cw20Coin, Logo, MinterResponse};
use cw20_base::msg::{
    ExecuteMsg as Cw20ExecuteMsg, InstantiateMarketingInfo, InstantiateMsg as Cw20InstantiateMsg,
    QueryMsg as Cw20QueryMsg,
};
use cw_utils::Expiration;

#[andr_instantiate]
#[andr_instantiate_modules]
#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: Option<MinterResponse>,
    pub marketing: Option<InstantiateMarketingInfo>,
}

impl From<InstantiateMsg> for Cw20InstantiateMsg {
    fn from(msg: InstantiateMsg) -> Self {
        Cw20InstantiateMsg {
            name: msg.name,
            symbol: msg.symbol,
            decimals: msg.decimals,
            initial_balances: msg.initial_balances,
            mint: msg.mint,
            marketing: msg.marketing,
        }
    }
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    /// Transfer is a base message to move tokens to another account without triggering actions
    Transfer {
        recipient: AndrAddr,
        amount: Uint128,
    },
    /// Burn is a base message to destroy tokens forever
    Burn { amount: Uint128 },
    /// Send is a base message to transfer tokens to a contract and trigger an action
    /// on the receiving contract.
    Send {
        contract: AndrAddr,
        amount: Uint128,
        msg: Binary,
    },
    /// Only with "approval" extension. Allows spender to access an additional amount tokens
    /// from the owner's (env.sender) account. If expires is Some(), overwrites current allowance
    /// expiration with this one.
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Only with "approval" extension. Lowers the spender's access of tokens
    /// from the owner's (env.sender) account by amount. If expires is Some(), overwrites current
    /// allowance expiration with this one.
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Only with "approval" extension. Transfers amount tokens from owner -> recipient
    /// if `env.sender` has sufficient pre-approval.
    TransferFrom {
        owner: String,
        recipient: AndrAddr,
        amount: Uint128,
    },
    /// Only with "approval" extension. Sends amount tokens from owner -> contract
    /// if `env.sender` has sufficient pre-approval.
    SendFrom {
        owner: String,
        contract: AndrAddr,
        amount: Uint128,
        msg: Binary,
    },
    /// Only with "approval" extension. Destroys tokens forever
    BurnFrom { owner: String, amount: Uint128 },
    /// Only with the "mintable" extension. If authorized, creates amount new tokens
    /// and adds to the recipient balance.
    Mint { recipient: String, amount: Uint128 },
    /// Only with the "marketing" extension. If authorized, updates marketing metadata.
    /// Setting None/null for any of these will leave it unchanged.
    /// Setting Some("") will clear this field on the contract storage
    UpdateMarketing {
        /// A URL pointing to the project behind this token.
        project: Option<String>,
        /// A longer description of the token and it's utility. Designed for tooltips or such
        description: Option<String>,
        /// The address (if any) who can update this data structure
        marketing: Option<String>,
    },
    /// If set as the "marketing" role on the contract, upload a new URL, SVG, or PNG for the token
    UploadLogo(Logo),
}

impl From<ExecuteMsg> for Cw20ExecuteMsg {
    fn from(msg: ExecuteMsg) -> Self {
        match msg {
            ExecuteMsg::Transfer { recipient, amount } => Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount,
            },
            ExecuteMsg::Burn { amount } => Cw20ExecuteMsg::Burn { amount },
            ExecuteMsg::Send {
                contract,
                amount,
                msg,
            } => Cw20ExecuteMsg::Send {
                contract: contract.to_string(),
                amount,
                msg,
            },
            ExecuteMsg::IncreaseAllowance {
                spender,
                amount,
                expires,
            } => Cw20ExecuteMsg::IncreaseAllowance {
                spender,
                amount,
                expires,
            },
            ExecuteMsg::DecreaseAllowance {
                spender,
                amount,
                expires,
            } => Cw20ExecuteMsg::DecreaseAllowance {
                spender,
                amount,
                expires,
            },
            ExecuteMsg::TransferFrom {
                owner,
                recipient,
                amount,
            } => Cw20ExecuteMsg::TransferFrom {
                owner,
                recipient: recipient.to_string(),
                amount,
            },
            ExecuteMsg::SendFrom {
                owner,
                contract,
                amount,
                msg,
            } => Cw20ExecuteMsg::SendFrom {
                owner,
                contract: contract.to_string(),
                amount,
                msg,
            },
            ExecuteMsg::BurnFrom { owner, amount } => Cw20ExecuteMsg::BurnFrom { owner, amount },
            ExecuteMsg::Mint { recipient, amount } => Cw20ExecuteMsg::Mint { recipient, amount },
            ExecuteMsg::UpdateMarketing {
                project,
                description,
                marketing,
            } => Cw20ExecuteMsg::UpdateMarketing {
                project,
                description,
                marketing,
            },
            ExecuteMsg::UploadLogo(logo) => Cw20ExecuteMsg::UploadLogo(logo),
            _ => panic!("Unsupported message"),
        }
    }
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    //NOTE: Balance is included in andr_query
    /// Returns the current balance of the given address, 0 if unset.
    /// Return type: BalanceResponse.
    // #[returns(BalanceResponse)]
    // Balance { address: AndrAddr },
    /// Returns metadata on the contract - name, decimals, supply, etc.
    /// Return type: TokenInfoResponse.
    #[returns(cw20::TokenInfoResponse)]
    TokenInfo {},
    /// Only with "mintable" extension.
    /// Returns who can mint and the hard cap on maximum tokens after minting.
    /// Return type: MinterResponse.
    #[returns(cw20::MinterResponse)]
    Minter {},
    /// Only with "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    /// Return type: AllowanceResponse.
    #[returns(cw20::AllowanceResponse)]
    Allowance { owner: String, spender: String },
    /// Only with "enumerable" extension (and "allowances")
    /// Returns all allowances this owner has approved. Supports pagination.
    /// Return type: AllAllowancesResponse.
    #[returns(cw20::AllAllowancesResponse)]
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "enumerable" extension
    /// Returns all accounts that have balances. Supports pagination.
    /// Return type: AllAccountsResponse.
    #[returns(cw20::AllAccountsResponse)]
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "marketing" extension
    /// Returns more metadata on the contract to display in the client:
    /// - description, logo, project url, etc.
    /// Return type: MarketingInfoResponse
    #[returns(cw20::MarketingInfoResponse)]
    MarketingInfo {},
    /// Only with "marketing" extension
    /// Downloads the mbeded logo data (if stored on chain). Errors if no logo data ftored for this
    /// contract.
    /// Return type: DownloadLogoResponse.
    #[returns(cw20::DownloadLogoResponse)]
    DownloadLogo {},
    #[returns(cw20::BalanceResponse)]
    Balance { address: String },
}

impl From<QueryMsg> for Cw20QueryMsg {
    fn from(msg: QueryMsg) -> Self {
        match msg {
            QueryMsg::Balance { address } => Cw20QueryMsg::Balance { address },
            QueryMsg::TokenInfo {} => Cw20QueryMsg::TokenInfo {},
            QueryMsg::Minter {} => Cw20QueryMsg::Minter {},
            QueryMsg::Allowance { owner, spender } => Cw20QueryMsg::Allowance { owner, spender },
            QueryMsg::AllAllowances {
                owner,
                start_after,
                limit,
            } => Cw20QueryMsg::AllAllowances {
                owner,
                start_after,
                limit,
            },
            QueryMsg::AllAccounts { start_after, limit } => {
                Cw20QueryMsg::AllAccounts { start_after, limit }
            }
            QueryMsg::MarketingInfo {} => Cw20QueryMsg::MarketingInfo {},
            QueryMsg::DownloadLogo {} => Cw20QueryMsg::DownloadLogo {},
            _ => panic!("Unsupported Msg"),
        }
    }
}
