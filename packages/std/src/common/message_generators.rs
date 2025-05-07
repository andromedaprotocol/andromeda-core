use crate::{amp::messages::AMPMsgConfig, error::ContractError};
use cosmwasm_std::{attr, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, SubMsg, WasmMsg};
use cw20::Cw20ExecuteMsg;

/// Creates a CW20 send message with an attached message payload and its associated attributes.
///
/// This differs from `create_cw20_transfer_msg` in that it sends tokens to a contract with an
/// attached message that the receiving contract can process.
///
/// # Arguments
///
/// * `contract` - The contract address that will receive the tokens and process the message
/// * `token_addr` - The CW20 token contract address
/// * `amount` - The amount of tokens to send
/// * `msg` - The message to attach to the token transfer
/// * `config` - Optional configuration for the submessage (gas limit, reply behavior)
///
/// # Returns
///
/// Returns a tuple containing:
/// * `SubMsg` - A submessage configured to send the CW20 tokens with the specified message
/// * `Vec<Attribute>` - A vector of attributes documenting the token send
pub fn create_cw20_send_msg(
    contract: &Addr,
    token_addr: &str,
    amount: u128,
    msg: Binary,
    config: AMPMsgConfig,
    id: u64,
) -> Result<(SubMsg, Vec<cosmwasm_std::Attribute>), ContractError> {
    let send_msg = Cw20ExecuteMsg::Send {
        contract: contract.to_string(),
        amount: amount.into(),
        msg: msg.clone(),
    };

    let sub_msg = SubMsg {
        id,
        reply_on: config.reply_on,
        gas_limit: config.gas_limit,
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_addr.to_string(),
            msg: to_json_binary(&send_msg)?,
            funds: vec![],
        }),
        payload: Binary::default(),
    };

    let attrs = vec![
        attr("token_send", format!("{amount} {token_addr}")),
        attr("contract", contract.to_string()),
    ];

    Ok((sub_msg, attrs))
}

/// Creates a bank send message and its associated attributes for transferring funds to a recipient.
///
/// # Arguments
///
/// * `recipient` - The address that will receive the funds
/// * `funds` - A slice of `Coin`s representing the funds to be sent
///
/// # Returns
///
/// Returns a tuple containing:
/// * `SubMsg` - A submessage configured to send the funds with error reply handling
/// * `Vec<Attribute>` - A vector of attributes documenting the funds transfer, including:
///   * One attribute per coin showing the amount and denomination
///   * One attribute showing the recipient address
pub fn create_bank_send_msg(
    recipient: &Addr,
    funds: &[Coin],
    id: u64,
) -> (SubMsg, Vec<cosmwasm_std::Attribute>) {
    let bank_msg = SubMsg::reply_on_error(
        CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: funds.to_vec(),
        }),
        id,
    );

    let attrs = funds
        .iter()
        .enumerate()
        .map(|(idx, fund)| attr(format!("funds:{idx}"), fund.to_string()))
        .chain(std::iter::once(attr("recipient", recipient.to_string())))
        .collect();

    (bank_msg, attrs)
}

/// Creates a CW20 token transfer message and its associated attributes.
///
/// # Arguments
///
/// * `recipient` - The address that will receive the tokens
/// * `token_addr` - The CW20 token contract address
/// * `amount` - The amount of tokens to transfer
///
/// # Returns
///
/// Returns a tuple containing:
/// * `SubMsg` - A submessage configured to send the CW20 tokens with error reply handling
/// * `Vec<Attribute>` - A vector of attributes documenting the token transfer
pub fn create_cw20_transfer_msg(
    recipient: &Addr,
    token_addr: &str,
    amount: u128,
    id: u64,
) -> Result<(SubMsg, Vec<cosmwasm_std::Attribute>), ContractError> {
    let transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: recipient.to_string(),
        amount: amount.into(),
    };

    let sub_msg = SubMsg::reply_on_error(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_addr.to_string(),
            msg: to_json_binary(&transfer_msg)?,
            funds: vec![],
        }),
        id,
    );

    let attrs = vec![
        attr("token_transfer", format!("{amount} {token_addr}")),
        attr("recipient", recipient.to_string()),
    ];

    Ok((sub_msg, attrs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::reply::ReplyId;
    use cosmwasm_std::ReplyOn;
    use rstest::rstest;

    #[rstest]
    #[case(
        Addr::unchecked("recipient"),
        vec![Coin::new(100u128, "uusd")],
        1
    )]
    #[case(
        Addr::unchecked("recipient2"),
        vec![Coin::new(100u128, "uusd"), Coin::new(200u128, "uluna")],
        2
    )]
    #[case(
        Addr::unchecked("recipient3"),
        vec![],
        0
    )]
    fn test_create_bank_send_msg(
        #[case] recipient: Addr,
        #[case] funds: Vec<Coin>,
        #[case] expected_fund_attrs: usize,
    ) {
        let (submsg, attrs) = create_bank_send_msg(&recipient, &funds, ReplyId::AMPMsg.repr());

        // Check the SubMsg
        match submsg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, recipient.to_string());
                assert_eq!(amount, funds);
            }
            _ => panic!("Expected BankMsg::Send"),
        }
        assert_eq!(submsg.id, ReplyId::AMPMsg.repr());
        assert_eq!(submsg.reply_on, cosmwasm_std::ReplyOn::Error);

        // Check attributes
        assert_eq!(attrs.len(), expected_fund_attrs + 1); // +1 for recipient attribute
        assert_eq!(attrs.last().unwrap().key, "recipient");
        assert_eq!(attrs.last().unwrap().value, recipient.to_string());

        // Check fund attributes
        for (idx, fund) in funds.iter().enumerate() {
            assert_eq!(attrs[idx].key, format!("funds:{idx}"));
            assert_eq!(attrs[idx].value, fund.to_string());
        }
    }

    #[rstest]
    #[case(Addr::unchecked("recipient"), "cw20_token_addr", 1000u128)]
    #[case(Addr::unchecked("recipient2"), "another_token", 500u128)]
    fn test_create_cw20_transfer_msg(
        #[case] recipient: Addr,
        #[case] token_addr: &str,
        #[case] amount: u128,
    ) {
        let (submsg, attrs) =
            create_cw20_transfer_msg(&recipient, token_addr, amount, ReplyId::AMPMsg.repr())
                .unwrap();

        // Check the SubMsg
        match submsg.msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) => {
                assert_eq!(contract_addr, token_addr);
                assert!(funds.is_empty());

                let decoded_msg: Cw20ExecuteMsg = cosmwasm_std::from_json(&msg).unwrap();
                match decoded_msg {
                    Cw20ExecuteMsg::Transfer {
                        recipient: msg_recipient,
                        amount: msg_amount,
                    } => {
                        assert_eq!(msg_recipient, recipient.to_string());
                        assert_eq!(msg_amount.u128(), amount);
                    }
                    _ => panic!("Expected Transfer message"),
                }
            }
            _ => panic!("Expected Wasm Execute message"),
        }
        assert_eq!(submsg.id, ReplyId::AMPMsg.repr());
        assert_eq!(submsg.reply_on, cosmwasm_std::ReplyOn::Error);

        // Check attributes
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].key, "token_transfer");
        assert_eq!(attrs[0].value, format!("{amount} {token_addr}"));
        assert_eq!(attrs[1].key, "recipient");
        assert_eq!(attrs[1].value, recipient.to_string());
    }

    #[rstest]
    #[case(
        Addr::unchecked("contract"),
        "cw20_token",
        1000u128,
        Binary::from(b"test_message"),
        AMPMsgConfig {
            reply_on: ReplyOn::Error,
            gas_limit: None,
            exit_at_error: false,
            direct: false,
            ibc_config: None,
        }
    )]
    #[case(
        Addr::unchecked("contract2"),
        "another_token",
        500u128,
        Binary::from(b"another_message"),
        AMPMsgConfig {
            reply_on: ReplyOn::Always,
            gas_limit: Some(1000000),
            exit_at_error: true,
            direct: false,
            ibc_config: None,
        }
    )]
    fn test_create_cw20_send_msg(
        #[case] contract: Addr,
        #[case] token_addr: &str,
        #[case] amount: u128,
        #[case] msg: Binary,
        #[case] config: AMPMsgConfig,
    ) {
        let (submsg, attrs) = create_cw20_send_msg(
            &contract,
            token_addr,
            amount,
            msg.clone(),
            config.clone(),
            ReplyId::AMPMsg.repr(),
        )
        .unwrap();

        // Check the SubMsg configuration
        assert_eq!(submsg.id, ReplyId::AMPMsg.repr());
        assert_eq!(submsg.reply_on, config.reply_on);
        assert_eq!(submsg.gas_limit, config.gas_limit);

        // Check the message content
        match submsg.msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: encoded_msg,
                funds,
            }) => {
                assert_eq!(contract_addr, token_addr);
                assert!(funds.is_empty());

                let decoded_msg: Cw20ExecuteMsg = cosmwasm_std::from_json(&encoded_msg).unwrap();
                match decoded_msg {
                    Cw20ExecuteMsg::Send {
                        contract: msg_contract,
                        amount: msg_amount,
                        msg: attached_msg,
                    } => {
                        assert_eq!(msg_contract, contract.to_string());
                        assert_eq!(msg_amount.u128(), amount);
                        assert_eq!(attached_msg, msg);
                    }
                    _ => panic!("Expected Send message"),
                }
            }
            _ => panic!("Expected Wasm Execute message"),
        }

        // Check attributes
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].key, "token_send");
        assert_eq!(attrs[0].value, format!("{amount} {token_addr}"));
        assert_eq!(attrs[1].key, "contract");
        assert_eq!(attrs[1].value, contract.to_string());
    }
}
