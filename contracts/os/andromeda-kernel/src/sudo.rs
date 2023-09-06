use andromeda_std::error::ContractError;
use cosmwasm_std::{DepsMut, Response};

pub mod ibc_lifecycle {
    // As with most IBC Hooks methods these were adapted from:
    // https://github.com/osmosis-labs/osmosis/blob/main/cosmwasm/contracts/crosschain-swaps/src/ibc_lifecycle.rs
    use cosmwasm_std::Coin;

    use crate::state::{OutgoingPacket, IBC_FUND_RECOVERY, OUTGOING_IBC_PACKETS};

    use super::*;

    pub fn receive_ack(
        deps: DepsMut,
        source_channel: String,
        sequence: u64,
        _ack: String,
        success: bool,
    ) -> Result<Response, ContractError> {
        let response = Response::new().add_attribute("action", "receive_ack");

        // Check if there is an inflight packet for the received (channel, sequence)
        let sent_packet =
            OUTGOING_IBC_PACKETS.may_load(deps.storage, (&source_channel, sequence))?;
        let Some(inflight_packet) = sent_packet else {
            // If there isn't, continue
            return Ok(response.add_attribute("msg", "received unexpected ack"))
        };
        OUTGOING_IBC_PACKETS.remove(deps.storage, (&source_channel, sequence));

        if success {
            // If the ack was successful, continue
            return Ok(response.add_attribute("msg", "received successful ack"));
        };

        let OutgoingPacket {
            recovery_addr,
            amount,
        } = inflight_packet;
        IBC_FUND_RECOVERY.update(deps.storage, &recovery_addr, |cur_amount_opt| {
            let mut recoveries = cur_amount_opt.unwrap_or_default();
            recoveries.push(amount.clone());
            Ok::<Vec<Coin>, ContractError>(recoveries)
        })?;

        Ok(response
            .add_attribute("msg", "msg failed")
            .add_attribute("recovery_addr", recovery_addr)
            .add_attribute("recovery_amount", amount.to_string()))
    }

    pub fn receive_timeout(
        deps: DepsMut,
        source_channel: String,
        sequence: u64,
    ) -> Result<Response, ContractError> {
        let response = Response::new().add_attribute("action", "receive_timeout");

        // Check if there is an inflight packet for the received (channel, sequence)
        let sent_packet =
            OUTGOING_IBC_PACKETS.may_load(deps.storage, (&source_channel, sequence))?;
        let Some(inflight_packet) = sent_packet else {
            // If there isn't, continue
            return Ok(response.add_attribute("msg", "received unexpected timeout"))
        };
        // Remove the in-flight packet
        OUTGOING_IBC_PACKETS.remove(deps.storage, (&source_channel, sequence));

        let OutgoingPacket {
            recovery_addr,
            amount,
        } = inflight_packet;
        IBC_FUND_RECOVERY.update(deps.storage, &recovery_addr, |cur_amount_opt| {
            let mut recoveries = cur_amount_opt.unwrap_or_default();
            recoveries.push(amount.clone());
            Ok::<Vec<Coin>, ContractError>(recoveries)
        })?;

        Ok(response
            .add_attribute("recovery_addr", recovery_addr)
            .add_attribute("recovery_amount", amount.to_string()))
    }
}
