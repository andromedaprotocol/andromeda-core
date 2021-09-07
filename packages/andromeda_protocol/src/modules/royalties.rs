use cosmwasm_std::{coin, Coin, DepsMut, Env, Event, MessageInfo, StdResult, Uint128};

use super::{
    common::{add_payment, deduct_payment},
    hooks::{HookResponse, MessageHooks, PaymentAttribute, ATTR_DEDUCTED, ATTR_DESC, ATTR_PAYMENT},
    Fee, Module, ModuleDefinition,
};

pub struct Royalty {
    pub fee: Fee,
    pub receivers: Vec<String>,
    pub description: Option<String>,
}

impl Royalty {
    pub fn calculate_fee(&self, payment: Coin) -> Coin {
        let fee_amount = payment
            .amount
            .multiply_ratio(Uint128::from(self.fee), 100 as u128);

        coin(fee_amount.u128(), payment.denom)
    }
}

impl MessageHooks for Royalty {
    fn on_agreed_transfer(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        _env: Env,
        payments: &mut Vec<cosmwasm_std::BankMsg>,
        owner: String,
        _purchaser: String,
        amount: cosmwasm_std::Coin,
    ) -> StdResult<HookResponse> {
        let fee_payment = self.calculate_fee(amount);
        let mut resp = HookResponse::default();
        let mut event = Event::new("royalty");

        match self.description.clone() {
            Some(desc) => {
                event = event.add_attribute(ATTR_DESC, desc);
            }
            None => {}
        }

        for receiver in self.receivers.to_vec() {
            deduct_payment(payments, owner.clone(), fee_payment.clone())?;
            event = event.add_attribute(ATTR_DEDUCTED, fee_payment.to_string());
            add_payment(payments, receiver.clone(), fee_payment.clone());
            event = event.add_attribute(
                ATTR_PAYMENT,
                PaymentAttribute {
                    receiver: receiver.clone(),
                    amount: fee_payment.clone(),
                }
                .to_string(),
            );
        }

        resp = resp.add_event(event);

        Ok(resp)
    }
}

impl Module for Royalty {
    fn validate(&self, _extensions: Vec<super::ModuleDefinition>) -> StdResult<bool> {
        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Royalties {
            fee: self.fee,
            receivers: self.receivers.to_vec(),
            description: self.description.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        BankMsg,
    };

    use super::*;

    #[test]
    fn test_on_agreed_transfer() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("purchaser", &[]);
        let env = mock_env();
        let owner = "owner";
        let receiver_one = "receiverone";
        let receiver_two = "receivertwo";
        let agreed_amount = coin(100, "uluna");
        let fee_amount = coin(2, "uluna");
        let mut payments = vec![BankMsg::Send {
            to_address: owner.to_string(),
            amount: vec![agreed_amount.clone()],
        }];
        let royalty = Royalty {
            fee: 2,
            receivers: vec![receiver_one.to_string(), receiver_two.to_string()],
            description: None,
        };

        royalty
            .on_agreed_transfer(
                &deps.as_mut(),
                info,
                env.clone(),
                &mut payments,
                owner.to_string(),
                String::default(),
                agreed_amount.clone(),
            )
            .unwrap();

        assert_eq!(payments.len(), 3);
        let receiver_one_payment = BankMsg::Send {
            to_address: receiver_one.to_string(),
            amount: vec![fee_amount.clone()],
        };
        assert_eq!(payments[1], receiver_one_payment);
        let receiver_two_payment = BankMsg::Send {
            to_address: receiver_two.to_string(),
            amount: vec![fee_amount.clone()],
        };
        assert_eq!(payments[2], receiver_two_payment);
        let deducted_payment = BankMsg::Send {
            to_address: owner.to_string(),
            amount: vec![coin(96, "uluna")],
        };
        assert_eq!(payments[0], deducted_payment);
    }

    #[test]
    fn test_on_agreed_transfer_resp() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("purchaser", &[]);
        let env = mock_env();
        let desc = "Some royalty description";
        let owner = "owner";
        let receiver_one = "receiverone";
        let receiver_two = "receivertwo";
        let agreed_amount = coin(100, "uluna");
        let mut payments = vec![BankMsg::Send {
            to_address: owner.to_string(),
            amount: vec![agreed_amount.clone()],
        }];
        let royalty = Royalty {
            fee: 2,
            receivers: vec![receiver_one.to_string(), receiver_two.to_string()],
            description: Some(desc.to_string()),
        };

        let resp = royalty
            .on_agreed_transfer(
                &deps.as_mut(),
                info,
                env.clone(),
                &mut payments,
                owner.to_string(),
                String::default(),
                agreed_amount.clone(),
            )
            .unwrap();

        assert_eq!(resp.events.len(), 1);
        assert_eq!(resp.events[0].ty, "royalty");
        assert_eq!(
            resp.events[0].attributes.len(),
            1 + (royalty.receivers.len() * 2)
        );
        assert_eq!(resp.events[0].attributes[0].key, ATTR_DESC);
        assert_eq!(resp.events[0].attributes[0].value, desc.to_string());
        assert_eq!(resp.events[0].attributes[1].key, ATTR_DEDUCTED);
        assert_eq!(
            resp.events[0].attributes[1].value,
            royalty.calculate_fee(agreed_amount.clone()).to_string()
        );
        assert_eq!(resp.events[0].attributes[2].key, ATTR_PAYMENT);
        assert_eq!(
            resp.events[0].attributes[2].value,
            PaymentAttribute {
                receiver: royalty.receivers[0].clone(),
                amount: royalty.calculate_fee(agreed_amount.clone())
            }
            .to_string()
        );
    }
}
