use andromeda_std::os::adodb::ADOVersion;
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, Executor};

use crate::{
    mock::{mock_app, MockApp},
    MockAndromeda,
};

pub struct MockAndromedaBuilder {
    app: MockApp,
    contracts: Vec<(u64, ADOVersion)>,
    admin_address: Addr,
}

impl MockAndromedaBuilder {
    pub fn new(denoms: Option<Vec<&str>>) -> Self {
        let app = mock_app(denoms.clone());

        Self {
            app,
            contracts: vec![],
            admin_address: Addr::unchecked("admin"),
        }
    }

    /// **THIS WILL CREATE A NEW MOCKANDROMEDA**
    pub fn with_app(self, app: MockApp) -> Self {
        Self {
            app,
            contracts: vec![],
            ..self
        }
    }

    pub fn with_balance(&mut self, addr: &Addr, amount: &[Coin]) -> &Self {
        self.app
            .send_tokens(Addr::unchecked("bank"), addr.clone(), amount)
            .unwrap();
        self
    }

    pub fn with_balances(&mut self, balances: &[(Addr, &[Coin])]) -> &Self {
        for (addr, amount) in balances {
            self.with_balance(addr, amount);
        }
        self
    }

    pub fn with_contract(
        &mut self,
        contract: Box<dyn Contract<Empty>>,
        version: ADOVersion,
    ) -> &Self {
        let code_id = self.app.store_code(contract);
        self.contracts.push((code_id, version));
        self
    }

    pub fn with_contracts(
        &mut self,
        contracts: Vec<(Box<dyn Contract<Empty>>, ADOVersion)>,
    ) -> &Self {
        for (contract, name) in contracts {
            self.with_contract(contract, name);
        }
        self
    }

    pub fn build(&mut self) -> MockAndromeda {
        let mut andr = MockAndromeda::new(&mut self.app, &self.admin_address);
        for (code_id, version) in self.contracts.iter() {
            andr.store_code_id(version.as_str(), *code_id);
        }

        andr
    }
}
