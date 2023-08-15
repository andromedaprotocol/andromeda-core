use crate::proto::MsgTransfer;
use crate::{
    ack::{make_ack_fail, make_ack_success},
    contract::try_wasm_msg,
};
use andromeda_std::amp::{messages::AMPMsg, AndrAddr};
use andromeda_std::error::{ContractError, Never};
use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, Binary, Coin, DepsMut, Env, Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannel,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, Response, Timestamp,
};
use sha256::digest;

pub const IBC_VERSION: &str = "message-bridge-1";
pub const PACKET_LIFETIME: u64 = 604_800u64;

#[cw_serde]
pub enum IBCLifecycleComplete {
    #[serde(rename = "ibc_ack")]
    IBCAck {
        channel: String,
        sequence: u64,
        ack: String,
        success: bool,
    },
    #[serde(rename = "ibc_timeout")]
    IBCTimeout { channel: String, sequence: u64 },
}

#[cw_serde]
pub enum SudoMsg {
    #[serde(rename = "ibc_lifecycle_complete")]
    IBCLifecycleComplete(IBCLifecycleComplete),
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // As with ack above, nothing to do here. If we cared about
    // keeping track of state between the two chains then we'd want to
    // respond to this likely as it means that the packet in question
    // isn't going anywhere.
    Ok(IbcBasicResponse::new().add_attribute("method", "ibc_packet_timeout"))
}

pub fn receive_ack(
    _deps: DepsMut,
    _source_channel: String,
    _sequence: u64,
    _ack: String,
    success: bool,
) -> Result<Response, ContractError> {
    if success {
        Ok(Response::default().add_attribute("response", "success"))
    } else {
        Ok(Response::default().add_attribute("response", "failure"))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<Option<Ibc3ChannelOpenResponse>, ContractError> {
    validate_order_and_version(msg.channel(), msg.counterparty_version())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_connect(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    validate_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel = msg.channel().endpoint.channel_id.clone();

    Ok(IbcBasicResponse::new()
        .add_attribute("method", "ibc_channel_connect")
        .add_attribute("channel_id", channel))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let channel = msg.channel().endpoint.channel_id.clone();
    // Reset the state for the channel.
    Ok(IbcBasicResponse::new()
        .add_attribute("method", "ibc_channel_close")
        .add_attribute("channel", channel))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    // Regardless of if our processing of this packet works we need to
    // commit an ACK to the chain. As such, we wrap all handling logic
    // in a seprate function and on error write out an error ack.
    match do_ibc_packet_receive(deps, env, msg) {
        Ok(response) => Ok(response),
        Err(error) => Ok(IbcReceiveResponse::new()
            .add_attribute("method", "ibc_packet_receive")
            .add_attribute("error", error.to_string())
            .set_ack(make_ack_fail(error.to_string()))),
    }
}

pub fn do_ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    // The channel this packet is being relayed along on this chain.

    Ok(IbcReceiveResponse::default())
}

fn _execute_send_message(
    deps: DepsMut,
    recipient: String,
    message: Binary,
) -> Result<IbcReceiveResponse, ContractError> {
    let wasm_msg = try_wasm_msg(deps, recipient.clone(), message.clone())?;

    Ok(IbcReceiveResponse::new()
        .add_message(wasm_msg)
        .add_attribute("method", "execute_send_message")
        .add_attribute("message", message.to_string())
        .add_attribute("recipient", recipient)
        .set_ack(make_ack_success()))
}

pub fn validate_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<Option<Ibc3ChannelOpenResponse>, ContractError> {
    // We expect an unordered channel here. Ordered channels have the
    // property that if a message is lost the entire channel will stop
    // working until you start it again.
    ensure!(
        channel.order == IbcOrder::Unordered,
        ContractError::OrderedChannel {}
    );
    ensure!(
        channel.version == IBC_VERSION,
        ContractError::InvalidVersion {
            actual: channel.version.to_string(),
            expected: IBC_VERSION.to_string(),
        }
    );

    // Make sure that we're talking with a counterparty who speaks the
    // same "protocol" as us.
    //
    // For a connection between chain A and chain B being established
    // by chain A, chain B knows counterparty information during
    // `OpenTry` and chain A knows counterparty information during
    // `OpenAck`. We verify it when we have it but when we don't it's
    // alright.
    if let Some(counterparty_version) = counterparty_version {
        ensure!(
            counterparty_version == IBC_VERSION,
            ContractError::InvalidVersion {
                actual: counterparty_version.to_string(),
                expected: IBC_VERSION.to_string(),
            }
        );
    }

    Ok(Some(Ibc3ChannelOpenResponse {
        version: channel.version.to_string(),
    }))
}

// IBC transfer port
const TRANSFER_PORT: &str = "transfer";

fn generate_ibc_denom(channel: String, denom: String) -> String {
    let path = format!("{}/{}/{}", TRANSFER_PORT, channel, denom);
    format!("ibc/{}", digest(path).to_uppercase())
}

#[allow(clippy::too_many_arguments)]
pub fn generate_transfer_message(
    recipient: AndrAddr,
    message: Binary,
    funds: Coin,
    channel: String,
    from_addr: String,
    to_addr: String,
    time: Timestamp,
) -> Result<MsgTransfer, ContractError> {
    // Convert funds denom
    let new_denom = generate_ibc_denom(channel.clone(), funds.clone().denom);
    let new_coin = Coin::new(funds.amount.u128(), new_denom);
    let msg = AMPMsg::new(recipient, message, Some(vec![new_coin]));
    let serialized = msg.to_ibc_hooks_memo(to_addr.clone(), from_addr.clone());

    let ts = time.plus_seconds(PACKET_LIFETIME);

    Ok(MsgTransfer {
        source_port: TRANSFER_PORT.into(),
        source_channel: channel,
        token: Some(funds.into()),
        sender: from_addr,
        receiver: to_addr,
        timeout_height: None,
        timeout_timestamp: Some(ts.nanos()),
        memo: serialized,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ibc_denom() {
        let channel = "channel-141";
        let denom = "uosmo";

        let expected = "ibc/14F9BC3E44B8A9C1BE1FB08980FAB87034C9905EF17CF2F5008FC085218811CC";
        let res = generate_ibc_denom(channel.to_string(), denom.to_string());

        assert_eq!(expected, res)
    }
}
