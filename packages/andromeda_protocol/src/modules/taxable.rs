use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, StdError, StdResult};

use crate::{
    modules::common::{add_payment, calculate_fee, require},
    modules::hooks::MessageHooks,
    modules::{Fee, Module, ModuleDefinition},
};

pub struct Taxable {
    pub tax: Fee,
    pub receivers: Vec<String>,
}

impl Module for Taxable {
    fn validate(&self, _extensions: Vec<crate::modules::ModuleDefinition>) -> StdResult<bool> {
        require(
            self.receivers.len() > 0,
            StdError::generic_err("Cannot apply a tax with no receiving addresses"),
        )?;
        // require(self.tax > 0, StdError::generic_err("Tax must be non-zero"))?;
        match self.tax.clone() {
            Fee::Flat(rate) => {
                require(
                    rate.amount > 0,
                    StdError::generic_err("Tax must be non-zero"),
                )?;
            }
            Fee::Percent(rate) => {
                require(rate > 0, StdError::generic_err("Tax must be non-zero"))?;
            }
        }

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Taxable {
            tax: self.tax.clone(),
            receivers: self.receivers.clone(),
        }
    }
}

impl MessageHooks for Taxable {
    fn on_agreed_transfer(
        &self,
        _deps: &DepsMut,
        _info: MessageInfo,
        env: Env,
        payments: &mut Vec<cosmwasm_std::BankMsg>,
        _owner: String,
        _purchaser: String,
        agreed_payment: Coin,
    ) -> StdResult<bool> {
        let _contract_addr = env.contract.address;
        let tax_amount = calculate_fee(self.tax.clone(), agreed_payment);

        for receiver in self.receivers.to_vec() {
            add_payment(payments, receiver.clone(), tax_amount.clone());
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coin, coins,
        testing::{mock_dependencies, mock_env, mock_info},
        BankMsg,
    };

    use super::*;

    #[test]
    fn test_taxable_validate() {
        let t = Taxable {
            tax: Fee::Percent(2),
            receivers: vec![String::default()],
        };

        assert_eq!(t.validate(vec![]).unwrap(), true);

        let t_invalidtax = Taxable {
            tax: Fee::Percent(0),
            receivers: vec![String::default()],
        };

        assert_eq!(
            t_invalidtax.validate(vec![]).unwrap_err(),
            StdError::generic_err("Tax must be non-zero")
        );

        let t_invalidrecv = Taxable {
            tax: Fee::Percent(2),
            receivers: vec![],
        };

        assert_eq!(
            t_invalidrecv.validate(vec![]).unwrap_err(),
            StdError::generic_err("Cannot apply a tax with no receiving addresses")
        );
    }

    #[test]

    fn test_taxable_on_agreed_transfer() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("sender", &[]);
        let env = mock_env();
        let receivers = vec![String::from("recv1"), String::from("recv2")];
        let t = Taxable {
            tax: Fee::Percent(3),
            receivers: receivers.clone(),
        };

        let agreed_transfer_amount = coin(117, "uluna");
        let tax_amount = 4;
        let owner = String::from("owner");
        let purchaser = String::from("purchaser");
        let mut payments = vec![];

        t.on_agreed_transfer(
            &deps.as_mut(),
            info.clone(),
            env.clone(),
            &mut payments,
            owner.clone(),
            purchaser.clone(),
            agreed_transfer_amount.clone(),
        )
        .unwrap();

        assert_eq!(payments.len(), 2);

        let first_payment = BankMsg::Send {
            to_address: String::from("recv1"),
            amount: coins(tax_amount, &agreed_transfer_amount.denom.to_string()),
        };
        let second_payment = BankMsg::Send {
            to_address: String::from("recv2"),
            amount: coins(tax_amount, &agreed_transfer_amount.denom.to_string()),
        };

        assert_eq!(payments[0], first_payment);
        assert_eq!(payments[1], second_payment);
    }
}
