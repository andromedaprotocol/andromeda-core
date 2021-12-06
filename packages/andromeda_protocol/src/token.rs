use crate::modules::{
    common::calculate_fee, read_modules, receipt::get_receipt_module, ModuleDefinition, Rate,
};
use crate::require::require;
use cosmwasm_std::{
    attr, coin, Addr, BankMsg, Binary, BlockInfo, Coin, DepsMut, Env, Event, MessageInfo, Response,
    StdError, StdResult, Uint128,
};
use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
//Duplicate Approval struct from CW721-base contract: https://github.com/CosmWasm/cosmwasm-plus/blob/main/contracts/cw721-base/src/state.rs
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
    pub key: String,
    pub value: String,
    //The string used to display the attribute, if none is provided the `key` field can be used
    pub display_label: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenMetadata {
    pub data_type: MetadataType,
    //A URL to the token's source
    pub external_url: Option<String>,
    //A URL to any off-chain data relating to the token, the response from this URL should match the defined `data_type` of the token
    pub data_url: Option<String>,
    //On chain attributes related to the token (basic key/value)
    pub attributes: Option<Vec<MetadataAttribute>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Token {
    pub token_id: String,
    pub owner: String,
    pub name: String,
    pub publisher: String,
    pub description: Option<String>,
    pub approvals: Vec<Approval>,
    pub transfer_agreement: Option<TransferAgreement>,
    pub metadata: Option<TokenMetadata>,
    pub archived: bool,
    pub image: Option<String>,
    //The current price listing for the token
    pub pricing: Option<Coin>,
}

impl Token {
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
pub struct TransferAgreement {
    pub amount: Coin,
    pub purchaser: String,
}

impl TransferAgreement {
    pub fn generate_payment(&self, to_address: String) -> BankMsg {
        BankMsg::Send {
            to_address,
            amount: vec![self.amount.clone()],
        }
    }
    pub fn calculate_fee(&self, fee: Rate) -> Coin {
        let amount = self.amount.amount;
        let fee_amount = match fee {
            Rate::Flat(flat_rate) => {
                amount.multiply_ratio(flat_rate.amount, flat_rate.denom.parse::<u128>().unwrap())
            }
            Rate::Percent(fee) => amount.multiply_ratio(fee, 100 as u128),
        };

        coin(fee_amount.u128(), self.amount.denom.clone())
    }
    pub fn generate_fee_payment(&self, to_address: String, rate: Rate) -> BankMsg {
        BankMsg::Send {
            to_address,
            amount: vec![calculate_fee(rate, self.amount.clone())],
        }
    }
    pub fn generate_event(self) -> Event {
        Event::new("agreed_transfer").add_attributes(vec![
            attr("amount", self.amount.to_string()),
            attr("purchaser", self.purchaser.clone()),
        ])
    }
    /*
        Adds generated module events and messages to the response object
    */
    pub fn on_transfer(
        self,
        deps: &DepsMut,
        info: &MessageInfo,
        env: &Env,
        owner: String,
        res_in: Response,
    ) -> StdResult<Response> {
        let mut res = res_in.clone();
        let modules = read_modules(deps.storage)?;
        let payment_message = self.generate_payment(owner.clone());
        let mut payments = vec![payment_message];
        let mod_resp = modules.on_agreed_transfer(
            &deps,
            info.clone(),
            env.clone(),
            &mut payments,
            owner.clone(),
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
        let illegal_names = "BitcoinEthereumDogecoinShibaInuTetherAdminRoot".to_string();
        // Maybe changing to vectors is a better idea since
        let illegal_symbols = "ETHBTCUSDT".to_string();
        require(
            !illegal_names.contains(&self.name),
            StdError::generic_err("Name is disallowed"),
        )?;
        require(
            !illegal_symbols.contains(&self.symbol),
            StdError::generic_err("Symbol is disallowed"),
        )?;

        Ok(true)
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MintMsg {
    pub token_id: String,
    pub owner: String,
    pub name: String,
    pub image: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<TokenMetadata>,
    pub pricing: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Mint(MintMsg),
    TransferNft {
        recipient: String,
        token_id: String,
    },
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
    Revoke {
        spender: String,
        token_id: String,
    },
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll {
        operator: String,
    },
    Burn {
        token_id: String,
    },
    Archive {
        token_id: String,
    },
    TransferAgreement {
        token_id: String,
        denom: String,
        amount: Uint128,
        purchaser: String,
    },
    UpdatePricing {
        token_id: String,
        price: Option<Coin>,
    },
    UpdateOwner {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    OwnerOf {
        token_id: String,
    },
    ApprovedForAll {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    NumTokens {},
    NftInfo {
        token_id: String,
    },
    AllNftInfo {
        token_id: String,
    },
    ModuleInfo {},
    ContractInfo {},
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
pub struct ModuleContract {
    pub module: String,
    pub contract: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
