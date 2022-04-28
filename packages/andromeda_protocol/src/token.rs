use crate::modules::{
    common::calculate_fee, read_modules, receipt::get_receipt_module, ModuleDefinition, Rate,
};
use crate::require;
use cosmwasm_std::{
    attr, Addr, BankMsg, Binary, BlockInfo, Coin, DepsMut, Env, Event, MessageInfo, Response,
    StdError, StdResult, Uint128,
};
use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Duplicate Approval struct from CW721-base contract: https://github.com/CosmWasm/cosmwasm-plus/blob/main/contracts/cw721-base/src/state.rs
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// Enum used to define the type of metadata held by a token
pub enum MetadataType {
    Image,
    Video,
    Audio,
    Domain,
    Json,
    Other,
}

impl ToString for MetadataType {
    fn to_string(&self) -> String {
        match self {
            MetadataType::Image => String::from("Image"),
            MetadataType::Video => String::from("Video"),
            MetadataType::Audio => String::from("Audio"),
            MetadataType::Domain => String::from("Domain"),
            MetadataType::Json => String::from("Json"),
            MetadataType::Other => String::from("Other"),
        }
    }
}
// [TOK-02] Add approval function should have been here but maybe was removed or altered in alter commits.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MetadataAttribute {
    /// The key for the attribute
    pub key: String,
    /// The value for the attribute
    pub value: String,
    /// The string used to display the attribute, if none is provided the `key` field can be used
    pub display_label: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenMetadata {
    /// The metadata type
    pub data_type: MetadataType,
    /// A URL to the token's source
    pub external_url: Option<String>,
    /// A URL to any off-chain data relating to the token, the response from this URL should match the defined `data_type` of the token
    pub data_url: Option<String>,
    /// On chain attributes related to the token (basic key/value)
    pub attributes: Option<Vec<MetadataAttribute>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Token {
    /// The ID of the token (unique)
    pub token_id: String,
    /// The owner of the token
    pub owner: String,
    /// The name of the token
    pub name: String,
    /// The original publisher of the token (immutable)
    pub publisher: String,
    /// An optional description of the token
    pub description: Option<String>,
    /// Any assigned approvals for the token
    pub approvals: Vec<Approval>,
    /// The transfer agreement of the token (if it exists)
    pub transfer_agreement: Option<TransferAgreement>,
    /// The metadata of the token (if it exists)
    pub metadata: Option<TokenMetadata>,
    /// Whether the token is archived or not
    pub archived: bool,
    /// An image URI for the token
    pub image: Option<String>,
    /// The current price listing for the token
    pub pricing: Option<Coin>,
}

impl Token {
    /// Removes address approval for the token for a given address
    pub fn filter_approval(&mut self, spender: &Addr) {
        self.approvals = self
            .approvals
            .clone()
            .into_iter()
            .filter(|a| !a.spender.eq(spender))
            .collect();
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A struct used to represent an agreed transfer of a token. The `purchaser` may use the `Transfer` message for this token as long as funds are provided equalling the `amount` defined in the agreement.
pub struct TransferAgreement {
    /// The amount required for the purchaser to transfer ownership of the token
    pub amount: Coin,
    /// The address of the purchaser
    pub purchaser: String,
}

impl TransferAgreement {
    /// Generates a `BankMsg` for the amount defined in the transfer agreement to the provided address
    pub fn generate_payment(&self, to_address: String) -> BankMsg {
        BankMsg::Send {
            to_address,
            amount: vec![self.amount.clone()],
        }
    }
    /// Generates a `BankMsg` for a given `Rate` to a given address
    pub fn generate_fee_payment(&self, to_address: String, rate: Rate) -> BankMsg {
        BankMsg::Send {
            to_address,
            amount: vec![calculate_fee(rate, self.amount.clone())],
        }
    }
    /// Generates an event related to the agreed transfer of a token
    pub fn generate_event(self) -> Event {
        Event::new("agreed_transfer").add_attributes(vec![
            attr("amount", self.amount.to_string()),
            attr("purchaser", self.purchaser),
        ])
    }
    /// Generates payment messages for any fees required by the current contracts Modules. Generates a receipt for the agreed transfer if the current contract contains a `Receipt` module.
    pub fn on_transfer(
        self,
        deps: &DepsMut,
        info: &MessageInfo,
        env: &Env,
        owner: String,
        res_in: Response,
    ) -> StdResult<Response> {
        let mut res = res_in;
        let modules = read_modules(deps.storage)?;
        let payment_message = self.generate_payment(owner.clone());
        let mut payments = vec![payment_message];
        let mod_resp = modules.on_agreed_transfer(
            &deps,
            info.clone(),
            env.clone(),
            &mut payments,
            owner,
            self.purchaser.clone(),
            self.amount.clone(),
        )?;

        for payment in payments {
            res = res.add_message(payment);
        }

        for event in &mod_resp.events {
            res = res.add_event(event.clone());
        }
        res = res.add_event(self.generate_event());

        let recpt_opt = get_receipt_module(deps.storage)?;
        match recpt_opt {
            Some(recpt_mod) => {
                let recpt_msg =
                    recpt_mod.generate_receipt_message(deps.storage, res.events.clone())?;
                res = res.add_message(recpt_msg);
            }
            None => {}
        }

        Ok(res)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,

    /// The minter is the only one who can create new NFTs.
    /// This is designed for a base NFT that is controlled by an external program
    /// or contract. You will likely replace this with custom logic in custom NFTs
    pub minter: String,

    //The attached Andromeda modules
    pub modules: Vec<ModuleDefinition>,
}
impl InstantiateMsg {
    pub fn validate(&self) -> StdResult<bool> {
        // [TOK-01] Add illegal names symbols as much as you want
        let illegal_names = vec![
            "Bitcoin".to_string(),
            "Ethereum".to_string(),
            "Admin".to_string(),
            "Root".to_string(),
            "Tether".to_string(),
            "Dogecoin".to_string(),
            "Elon".to_string(),
            "Shiba Inu".to_string(),
        ];
        // Maybe changing to vectors is a better idea since
        let illegal_symbols = vec![
            "BTC".to_string(),
            "ETH".to_string(),
            "DOGE".to_string(),
            "USDT".to_string(),
        ];
        // Could also later on add a list of blacklist addresses that are not allowed to mint.
        let blacklist = vec!["blacklisted".to_string()];
        require(
            !illegal_names.contains(&self.name),
            StdError::generic_err("Name is illegal to be initialized."),
        )?;
        require(
            !illegal_symbols.contains(&self.symbol),
            StdError::generic_err("Symbol is illegal to be used."),
        )?;

        require(
            !blacklist.contains(&self.minter),
            StdError::generic_err("Minter is on blacklist. Not allowed to mint."),
        )?;

        Ok(true)
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintMsg {
    /// The ID of the token
    pub token_id: String,
    /// The owner of the token
    pub owner: String,
    /// The name of the token
    pub name: String,
    /// The image URI of the token
    pub image: Option<String>,
    /// An optional description of the token
    pub description: Option<String>,
    /// Any metadata related to the token
    pub metadata: Option<TokenMetadata>,
    /// The listing price of the token
    pub pricing: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Mints a token
    Mint(MintMsg),
    /// Transfers ownership of a token
    TransferNft { recipient: String, token_id: String },
    /// Sends a token to another contract
    SendNft {
        contract: String,
        token_id: String,
        msg: Binary,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke { spender: String, token_id: String },
    /// Approves an address for all tokens owned by the sender
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll { operator: String },
    /// Burns a token, removing all data related to it. The ID of the token is still reserved.
    Burn { token_id: String },
    /// Archives a token, causing it to be immutable but readable
    Archive { token_id: String },
    /// Assigns a `TransferAgreement` for a token
    TransferAgreement {
        token_id: String,
        denom: String,
        amount: Uint128,
        purchaser: String,
    },
    /// Updates the pricing of a token
    UpdatePricing {
        token_id: String,
        price: Option<Coin>,
    },
    /// Update ownership of the contract. Only executable by the current contract owner.
    UpdateOwner {
        /// The address of the new contract owner.
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Owner of the given token by ID
    OwnerOf { token_id: String },
    /// Approvals for a given address (paginated)
    ApprovedForAll {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Amount of tokens minted by the contract
    NumTokens {},
    /// The data of a token
    NftInfo { token_id: String },
    /// The data of a token and any approvals assigned to it
    AllNftInfo { token_id: String },
    /// Info of any modules assigned to the contract
    ModuleInfo {},
    /// The current config of the contract
    ContractInfo {},
    /// The current owner of the contract
    ContractOwner {},
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ArchivedResponse {
    pub archived: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ModulesResponse {
    pub modules: Vec<ModuleDefinition>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftInfoResponseExtension {
    pub metadata: Option<TokenMetadata>,
    pub archived: bool,
    pub pricing: Option<Coin>,
    pub transfer_agreement: Option<TransferAgreement>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ModuleInfoResponse {
    pub modules: Vec<ModuleDefinition>,
    pub contracts: Vec<ModuleContract>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// A struct used to defined a module and its current contract address (if it exists)
pub struct ModuleContract {
    /// The module name
    pub module: String,
    /// The contract address (if it exists)
    pub contract: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
