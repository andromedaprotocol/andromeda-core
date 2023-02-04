use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Addr, Empty, Env, IbcTimeout, StdResult, WasmMsg};
use cw721_proxy_derive::cw721_proxy;
use cw_cii::ContractInstantiateInfo;
use cw_pause_once::PauseOrchestrator;
use cw_storage_plus::{Item, Map};
use serde::Deserialize;

/// The code ID we will use for instantiating new cw721s.
pub const CW721_CODE_ID: Item<u64> = Item::new("cw721_code_id");
/// The proxy that this contract is receiving NFTs from, if any.
pub const PROXY: Item<Option<Addr>> = Item::new("proxy");
/// Manages contract pauses.
pub const PO: PauseOrchestrator = PauseOrchestrator::new("pauser_key", "paused_key");

/// Maps classID (from NonFungibleTokenPacketData) to the cw721
/// contract we have instantiated for that classID.
pub const CLASS_ID_TO_NFT_CONTRACT: Map<String, Addr> = Map::new("class_id_to_nft_contract");
/// Maps cw721 contracts to the classID they were instantiated for.
pub const NFT_CONTRACT_TO_CLASS_ID: Map<Addr, String> = Map::new("nft_contract_to_class_id");

/// Maps between classIDs and classUris. We need to keep this state
/// ourselves as cw721 contracts do not have class-level metadata.
pub const CLASS_ID_TO_CLASS_URI: Map<String, Option<String>> = Map::new("class_id_to_class_uri");

/// Maps (class ID, token ID) -> local channel ID. Used to determine
/// the local channel that NFTs have been sent out on.
pub const OUTGOING_CLASS_TOKEN_TO_CHANNEL: Map<(String, String), String> =
    Map::new("outgoing_class_token_to_channel");
/// Same as above, but for NFTs arriving at this contract.
pub const INCOMING_CLASS_TOKEN_TO_CHANNEL: Map<(String, String), String> =
    Map::new("incoming_class_token_to_channel");

#[derive(Deserialize)]
pub struct UniversalNftInfoResponse {
    pub token_uri: Option<String>,

    #[serde(skip_deserializing)]
    #[allow(dead_code)]
    extension: Empty,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_binary, to_binary, Coin, Empty};

    use super::UniversalNftInfoResponse;

    #[test]
    fn test_universal_deserialize() {
        let start = cw721::NftInfoResponse::<Coin> {
            token_uri: None,
            extension: Coin::new(100, "ujuno"),
        };
        let start = to_binary(&start).unwrap();
        let end: UniversalNftInfoResponse = from_binary(&start).unwrap();
        assert_eq!(end.token_uri, None);
        assert_eq!(end.extension, Empty::default())
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Code ID of cw721-ics contract. A new cw721-ics will be
    /// instantiated for each new IBCd NFT classID.
    ///
    /// NOTE: this _must_ correspond to the cw721-base contract. Using
    /// a regular cw721 may cause the ICS 721 interface implemented by
    /// this contract to stop working, and IBCd away NFTs to be
    /// unreturnable as cw721 does not have a mint method in the spec.
    pub cw721_base_code_id: u64,
    /// An optional proxy contract. If a proxy is set the contract
    /// will only accept NFTs from that proxy. The proxy is expected
    /// to implement the cw721 proxy interface defined in the
    /// cw721-proxy crate.
    pub proxy: Option<ContractInstantiateInfo>,
    /// Address that may pause the contract. PAUSER may pause the
    /// contract a single time; in pausing the contract they burn the
    /// right to do so again. A new pauser may be later nominated by
    /// the CosmWasm level admin via a migration.
    pub pauser: Option<String>,
}

#[cw721_proxy]
#[cw_serde]
pub enum ExecuteMsg {
    /// Receives a NFT to be IBC transfered away. The `msg` field must
    /// be a binary encoded `IbcOutgoingMsg`.
    ReceiveNft(cw721::Cw721ReceiveMsg),

    /// Pauses the bridge. Only the pauser may call this. In pausing
    /// the contract, the pauser burns the right to do so again.
    Pause {},

    /// Mesages used internally by the contract. These may only be
    /// called by the contract itself.
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum CallbackMsg {
    /// Mints a NFT of collection class_id for receiver with the
    /// provided id and metadata. Only callable by this contract.
    Mint {
        /// The class_id to mint for. This must have previously been
        /// created with `SaveClass`.
        class_id: String,
        /// Unique identifiers for the tokens.
        token_ids: Vec<String>,
        /// Urls pointing to metadata about the NFTs to mint. For
        /// example, this may point to ERC721 metadata on IPFS. Must
        /// be the same length as token_ids. token_uris[i] is the
        /// metadata for token_ids[i].
        token_uris: Vec<String>,
        /// The address that ought to receive the NFTs. This is a
        /// local address, not a bech32 public key.
        receiver: String,
    },
    /// Much like mint, but will instantiate a new cw721 contract iff
    /// the classID does not have one yet.
    InstantiateAndMint {
        /// The ics721 class ID to mint for.
        class_id: String,
        /// The URI for this class ID.
        class_uri: Option<String>,
        /// Unique identifiers for the tokens being transfered.
        token_ids: Vec<String>,
        /// A list of urls pointing to metadata about the NFTs. For
        /// example, this may point to ERC721 metadata on ipfs.
        ///
        /// Must be the same length as token_ids.
        token_uris: Vec<String>,
        /// The address that ought to receive the NFT. This is a local
        /// address, not a bech32 public key.
        receiver: String,
    },
    /// Transfers a number of NFTs identified by CLASS_ID and
    /// TOKEN_IDS to RECEIVER.
    BatchTransfer {
        /// The ics721 class ID of the tokens to be transfered.
        class_id: String,
        /// The address that should receive the tokens.
        receiver: String,
        /// The tokens (of CLASS_ID) that should be sent.
        token_ids: Vec<String>,
    },
    /// Handles the falliable part of receiving an IBC
    /// packet. Transforms TRANSFERS into a `BatchTransfer` message
    /// and NEW_TOKENS into a `DoInstantiateAndMint`, then dispatches
    /// those methods.
    HandlePacketReceive {
        /// The address receiving the NFTs.
        receiver: String,
        /// The URI for the collection being transfered.
        class_uri: Option<String>,
        /// Information about transfer actions.
        transfers: Option<TransferInfo>,
        /// Information about mint actions.
        new_tokens: Option<NewTokenInfo>,
    },
}

#[cw_serde]
pub struct TransferInfo {
    /// The class ID the tokens belong to.
    pub class_id: String,
    /// The tokens to be transfered.
    pub token_ids: Vec<String>,
}

impl TransferInfo {
    pub fn into_wasm_msg(self, env: &Env, receiver: &Addr) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::BatchTransfer {
                class_id: self.class_id,
                receiver: receiver.to_string(),
                token_ids: self.token_ids,
            }))?,
            funds: vec![],
        })
    }
}

#[cw_serde]
pub struct NewTokenInfo {
    /// The class ID to mint tokens for.
    pub class_id: String,
    /// The token IDs of the tokens to be minted.
    pub token_ids: Vec<String>,
    /// The URIs of the tokens to be minted. Matched with token_ids by
    /// index.
    pub token_uris: Vec<String>,
}

impl NewTokenInfo {
    pub fn into_wasm_msg(
        self,
        env: &Env,
        receiver: &Addr,
        class_uri: Option<String>,
    ) -> StdResult<WasmMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::InstantiateAndMint {
                class_id: self.class_id,
                class_uri,
                receiver: receiver.to_string(),
                token_ids: self.token_ids,
                token_uris: self.token_uris,
            }))?,
            funds: vec![],
        })
    }
}

#[cw_serde]
pub struct IbcOutgoingMsg {
    /// The address that should receive the NFT being sent on the
    /// *receiving chain*.
    pub receiver: String,
    /// The *local* channel ID this ought to be sent away on. This
    /// contract must have a connection on this channel.
    pub channel_id: String,
    /// Timeout for the IBC message.
    pub timeout: IbcTimeout,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Gets the classID this contract has stored for a given NFT
    /// contract. If there is no class ID for the provided contract,
    /// returns None.
    #[returns(Option<String>)]
    ClassId { contract: String },

    /// Gets the NFT contract associated wtih the provided class
    /// ID. If no such contract exists, returns None. Returns
    /// Option<Addr>.
    #[returns(Option<::cosmwasm_std::Addr>)]
    NftContract { class_id: String },

    /// Gets the class level metadata URI for the provided
    /// class_id. If there is no metadata, returns None. Returns
    /// `Option<String>`.
    #[returns(Option<String>)]
    Metadata { class_id: String },

    /// Gets the owner of the NFT identified by CLASS_ID and
    /// TOKEN_ID. Errors if no such NFT exists. Returns
    /// `cw721::OwnerOfResonse`.
    #[returns(::cw721::OwnerOfResponse)]
    Owner { class_id: String, token_id: String },

    /// Gets the address that may pause this contract if one is set.
    #[returns(Option<::cosmwasm_std::Addr>)]
    Pauser {},

    /// Gets the current pause status.
    #[returns(bool)]
    Paused {},

    /// Gets this contract's cw721-proxy if one is set.
    #[returns(Option<::cosmwasm_std::Addr>)]
    Proxy {},
}

#[cw_serde]
pub enum MigrateMsg {
    WithUpdate {
        /// The address that may pause the contract. If `None` is
        /// provided the current pauser will be removed.
        pauser: Option<String>,
        /// The cw721-proxy for this contract. If `None` is provided
        /// the current proxy will be removed.
        proxy: Option<String>,
    },
}

#[cw_serde]
#[serde(rename_all = "camelCase")]
pub struct NonFungibleTokenPacketData {
    /// Uniquely identifies the collection which the tokens being
    /// transfered belong to on the sending chain.
    pub class_id: String,
    /// URL that points to metadata about the collection. This is not
    /// validated.
    pub class_uri: Option<String>,
    /// Uniquely identifies the tokens in the NFT collection being
    /// transfered.
    pub token_ids: Vec<String>,
    /// URL that points to metadata for each token being
    /// transfered. `tokenUris[N]` should hold the metadata for
    /// `tokenIds[N]` and both lists should have the same length.
    pub token_uris: Vec<String>,
    /// The address sending the tokens on the sending chain.
    pub sender: String,
    /// The address that should receive the tokens on the receiving
    /// chain.
    pub receiver: String,
}
