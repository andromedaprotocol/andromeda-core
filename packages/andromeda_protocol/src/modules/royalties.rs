use cosmwasm_std::{DepsMut, Env, MessageInfo, StdResult};

use super::{
    common::{add_payment, calculate_fee, deduct_payment},
    hooks::MessageHooks,
    Module, ModuleDefinition, Rate,
};

pub struct Royalty {
    pub rate: Rate,
    pub receivers: Vec<String>,
    pub description: Option<String>,
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
    ) -> StdResult<bool> {
        let fee_payment = calculate_fee(self.rate.clone(), amount);
        for receiver in self.receivers.to_vec() {
            deduct_payment(payments, owner.clone(), fee_payment.clone())?;
            add_payment(payments, receiver, fee_payment.clone());
        }

        Ok(true)
    }
}

impl Module for Royalty {
    fn validate(&self, _modules: Vec<super::ModuleDefinition>) -> StdResult<bool> {
        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Royalties {
            rate: self.rate.clone(),
            receivers: self.receivers.to_vec(),
            description: self.description.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coin,
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
            rate: Rate::Percent(2),
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
}
