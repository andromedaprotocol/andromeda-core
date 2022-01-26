use cosmwasm_std::{MessageInfo, QuerierWrapper};

use crate::{
    address_list::on_execute,
    error::ContractError,
    rates::{on_required_payments, DeductedFundsResponse, Funds},
};

pub enum Module {
    Rates(String),
    Whitelist(String),
}

impl Module {
    pub fn on_execute(
        self,
        querier: QuerierWrapper,
        info: MessageInfo,
    ) -> Result<(), ContractError> {
        match self {
            Module::Whitelist(addr) => on_execute(querier, addr, info),
            _ => Ok(()),
        }
    }
    pub fn on_required_payments(
        self,
        querier: QuerierWrapper,
        amount: Funds,
    ) -> Result<Option<DeductedFundsResponse>, ContractError> {
        match self {
            Module::Rates(addr) => Ok(Some(on_required_payments(querier, addr, amount)?)),
            _ => Ok(None),
        }
    }
}
