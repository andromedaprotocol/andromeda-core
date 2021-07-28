use cosmwasm_std::{coin, Coin, Env, HumanAddr, StdError, StdResult, Uint128};

use crate::{
    common::{add_payment, require},
    hooks::{Payments, PreHooks},
    modules::{Fee, Module, ModuleDefinition},
};

struct Taxable {
    tax: Fee,
    receivers: Vec<HumanAddr>,
}

impl PreHooks for Taxable {}

impl Payments for Taxable {
    fn on_agreed_transfer(
        &self,
        env: Env,
        payments: &mut Vec<cosmwasm_std::BankMsg>,
        _owner: HumanAddr,
        _purchaser: HumanAddr,
        agreed_payment: Coin,
    ) -> StdResult<bool> {
        let contract_addr = env.contract.address;
        let tax_amount: Uint128 = agreed_payment
            .amount
            .multiply_ratio(Uint128(self.tax), 100 as u128);

        let tax = coin(tax_amount.u128(), &agreed_payment.denom.to_string());

        for receiver in self.receivers.to_vec() {
            add_payment(
                payments,
                contract_addr.clone(),
                receiver.clone(),
                tax.clone(),
            );
        }

        Ok(true)
    }
}

impl Module for Taxable {
    fn validate(&self, _extensions: Vec<crate::modules::ModuleDefinition>) -> StdResult<bool> {
        require(
            self.receivers.len() > 0,
            StdError::generic_err("Cannot apply a tax with no receiving addresses"),
        )?;
        require(self.tax > 0, StdError::generic_err("Tax must be non-zero"))?;

        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Taxable {
            tax: self.tax,
            receivers: self.receivers.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coins, testing::mock_env, BankMsg};

    use super::*;

    #[test]
    fn test_taxable_validate() {
        let t = Taxable {
            tax: 2,
            receivers: vec![HumanAddr::default()],
        };

        assert_eq!(t.validate(vec![]).unwrap(), true);

        let t_invalidtax = Taxable {
            tax: 0,
            receivers: vec![HumanAddr::default()],
        };

        assert_eq!(
            t_invalidtax.validate(vec![]).unwrap_err(),
            StdError::generic_err("Tax must be non-zero")
        );

        let t_invalidrecv = Taxable {
            tax: 2,
            receivers: vec![],
        };

        assert_eq!(
            t_invalidrecv.validate(vec![]).unwrap_err(),
            StdError::generic_err("Cannot apply a tax with no receiving addresses")
        );
    }

    #[test]

    fn test_taxable_on_agreed_transfer() {
        let env = mock_env(HumanAddr::default(), &coins(100, "uluna"));
        let receivers = vec![HumanAddr::from("recv1"), HumanAddr::from("recv2")];
        let t = Taxable {
            tax: 1,
            receivers: receivers.clone(),
        };

        let agreed_transfer_amount = coin(100, "uluna");
        let tax_amount = 1;
        let owner = HumanAddr::from("owner");
        let purchaser = HumanAddr::from("purchaser");
        let mut payments = vec![];

        t.on_agreed_transfer(
            env.clone(),
            &mut payments,
            owner.clone(),
            purchaser.clone(),
            agreed_transfer_amount.clone(),
        )
        .unwrap();

        assert_eq!(payments.len(), 2);

        let first_payment = BankMsg::Send {
            from_address: env.contract.address.clone(),
            to_address: HumanAddr::from("recv1"),
            amount: coins(tax_amount, &agreed_transfer_amount.denom.to_string()),
        };
        let second_payment = BankMsg::Send {
            from_address: env.contract.address.clone(),
            to_address: HumanAddr::from("recv2"),
            amount: coins(tax_amount, &agreed_transfer_amount.denom.to_string()),
        };

        assert_eq!(payments[0], first_payment);
        assert_eq!(payments[1], second_payment);
    }
}
