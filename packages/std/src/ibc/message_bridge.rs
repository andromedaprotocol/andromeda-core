use crate::amp::messages::{AMPMsg, AMPPkt};
use crate::amp::AndrAddr;
use crate::error::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Binary, Coin, Env, IbcMsg, IbcTimeout, MessageInfo, Timestamp};

#[cw_serde]
pub struct InstantiateMsg {
    pub kernel_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AMPReceive(AMPPkt),
    SendMessage {
        chain: String,
        recipient: AndrAddr,
        message: Binary,
    },
    /// Receives an AMPMsg, creates a new AMPPkt that contains the AMPMsg and sends it to the recipient
    SendAmpMessage {
        chain: String,
        recipient: AndrAddr,
        message: AMPMsg,
    },

    SendAmpPacket {
        chain: String,
        message: Vec<AMPMsg>,
    },
    SaveChannel {
        channel: String,
        chain: String,
        kernel_address: String,
    },
    UpdateChannel {
        channel: String,
        chain: String,
        kernel_address: Option<String>,
    },
}

#[cw_serde]
pub enum IbcExecuteMsg {
    SendMessage { recipient: String, message: Binary },
    SendAmpPacket { message: Binary },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    ChannelID { chain: String },
    #[returns(Vec<String>)]
    SupportedChains {},
}

/// Creates an IbcTransfer message for every AMP message
pub fn try_ibc_funds(
    env: Env,
    info: MessageInfo,
    amp_messages: Vec<AMPMsg>,
    channel: String,
) -> Result<Vec<IbcMsg>, ContractError> {
    let funds = &info.funds[0];

    let port = env.contract.address.to_string();
    let new_denom = format!("wasm.{}/{}/{}", port, channel, funds.denom);

    let mut transfer_msgs = vec![];
    for amp_msg in amp_messages {
        // We need to parse the recipient
        let recipient = amp_msg.recipient.get_raw_path();

        ensure!(
            !amp_msg.funds.is_empty(),
            ContractError::InsufficientFunds {}
        );
        let new_amount = amp_msg.funds[0].amount.u128();
        let new_coin = Coin::new(new_amount, new_denom.clone());

        let message = IbcMsg::Transfer {
            channel_id: channel.clone(),
            to_address: recipient.to_string(),
            amount: new_coin,
            timeout: IbcTimeout::with_timestamp(Timestamp::from_seconds(60)),
        };
        transfer_msgs.push(message);
    }
    Ok(transfer_msgs)
}
