use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    DepsMut, Env, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg,
    IbcChannelOpenMsg, IbcChannelOpenResponse, IbcOrder, IbcPacketAckMsg, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, Reply, Response, SubMsgResult,
};
use cw_storage_plus::Item;
use cw_utils::parse_reply_instantiate_data;

use crate::{
    ibc_helpers::{self, ack_fail, ack_success},
    ics721::{ClassId, CLASS_ID_TO_NFT_CONTRACT, NFT_CONTRACT_TO_CLASS_ID, PROXY},
};
use common::error::{ContractError, Never};

/// Submessage reply ID used for instantiating cw721 contracts.
pub const INSTANTIATE_CW721_REPLY_ID: u64 = 0;
/// Submessage reply ID used for instantiating the proxy contract.
pub const INSTANTIATE_PROXY_REPLY_ID: u64 = 1;
/// Submessages dispatched with this reply ID will set the ack on the
/// response depending on if the submessage execution succeded or
/// failed.
pub const ACK_AND_DO_NOTHING: u64 = 2;
/// The IBC version this contract expects to communicate with.
pub const IBC_VERSION: &str = "ics721-1";

#[cw_serde]
pub enum AckMode {
    // Messages should respond with an error ACK.
    Error,
    // Messages should respond with a success ACK.
    Success,
}

pub const ACK_MODE: Item<AckMode> = Item::new("ack_mode");
pub const LAST_ACK: Item<AckMode> = Item::new("ack_mode");

pub fn validate_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    // We expect an unordered channel here. Ordered channels have the
    // property that if a message is lost the entire channel will stop
    // working until you start it again.
    if channel.order != IbcOrder::Unordered {
        return Err(ContractError::OrderedChannel {});
    }

    if channel.version != IBC_VERSION {
        return Err(ContractError::InvalidVersion {
            actual: channel.version.to_string(),
            expected: IBC_VERSION.to_string(),
        });
    }

    // Make sure that we're talking with a counterparty who speaks the
    // same "protocol" as us.
    //
    // For a connection between chain A and chain B being established
    // by chain A, chain B knows counterparty information during
    // `OpenTry` and chain A knows counterparty information during
    // `OpenAck`. We verify it when we have it but when we don't it's
    // alright.
    if let Some(counterparty_version) = counterparty_version {
        if counterparty_version != IBC_VERSION {
            return Err(ContractError::InvalidVersion {
                actual: counterparty_version.to_string(),
                expected: IBC_VERSION.to_string(),
            });
        }
    }

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        INSTANTIATE_CW721_REPLY_ID => {
            // Don't need to add an ack or check for an error here as this
            // is only replies on success. This is OK because it is only
            // ever used in `DoInstantiateAndMint` which itself is always
            // a submessage of `ibc_packet_receive` which is caught and
            // handled correctly by the reply handler for
            // `ACK_AND_DO_NOTHING`.

            let res = parse_reply_instantiate_data(reply).unwrap();
            let cw721_addr = deps.api.addr_validate(&res.contract_address)?;

            // We need to map this address back to a class
            // ID. Fourtunately, we set the name of the new NFT
            // contract to the class ID.
            let cw721::ContractInfoResponse { name, .. } = deps
                .querier
                .query_wasm_smart(cw721_addr.clone(), &cw721::Cw721QueryMsg::ContractInfo {})?;
            let class_id = ClassId::new(name);

            // Save classId <-> contract mappings.
            CLASS_ID_TO_NFT_CONTRACT.save(deps.storage, class_id.clone(), &cw721_addr)?;
            NFT_CONTRACT_TO_CLASS_ID.save(deps.storage, cw721_addr.clone(), &class_id)?;

            Ok(Response::default()
                .add_attribute("method", "instantiate_cw721_reply")
                .add_attribute("class_id", class_id)
                .add_attribute("cw721_addr", cw721_addr))
        }
        INSTANTIATE_PROXY_REPLY_ID => {
            let res = parse_reply_instantiate_data(reply).unwrap();
            let proxy_addr = deps.api.addr_validate(&res.contract_address)?;
            PROXY.save(deps.storage, &Some(proxy_addr))?;

            Ok(Response::default()
                .add_attribute("method", "instantiate_proxy_reply_id")
                .add_attribute("proxy", res.contract_address))
        }
        // These messages don't need to do any state changes in the
        // reply - just need to commit an ack.
        ACK_AND_DO_NOTHING => {
            match reply.result {
                // On success, set a successful ack. Nothing else to do.
                SubMsgResult::Ok(_) => Ok(Response::new().set_data(ack_success())),
                // On error we need to use set_data to override the data field
                // from our caller, the IBC packet recv, and acknowledge our
                // failure.  As per:
                // https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#handling-the-reply
                SubMsgResult::Err(err) => Ok(Response::new().set_data(ack_fail(err))),
            }
        }
        _ => Err(ContractError::UnrecognisedReplyId {}),
    }
}
