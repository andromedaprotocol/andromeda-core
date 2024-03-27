#![cfg(all(not(target_arch = "wasm32")))]

use std::collections::HashMap;

use andromeda_adodb::mock::mock_andromeda_adodb;
use andromeda_economics::mock::mock_andromeda_economics;
use andromeda_kernel::mock::mock_andromeda_kernel;
use andromeda_vfs::mock::mock_andromeda_vfs;
use cosmwasm_std::{coin, Addr, Coin};
use cw_multi_test::{
    App, AppBuilder, BankKeeper, Executor, MockAddressGenerator, MockApiBech32, WasmKeeper,
};

use crate::{mock_contract::MockContract, MockADODB, MockEconomics, MockKernel, MockVFS};

pub const ADMIN_USERNAME: &str = "am";

pub type MockApp = App<BankKeeper, MockApiBech32>;

pub fn mock_app(denoms: Option<Vec<&str>>) -> MockApp {
    let denoms = denoms.unwrap_or(vec!["uandr", "uusd"]);
    AppBuilder::new()
        .with_api(MockApiBech32::new("andr"))
        .with_wasm(WasmKeeper::new().with_address_generator(MockAddressGenerator))
        .build(|router, _api, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked("bank"),
                    denoms
                        .iter()
                        .map(|d| coin(u128::MAX, *d))
                        .collect::<Vec<Coin>>(),
                )
                .unwrap();
        })
}

pub fn init_balances(app: &mut MockApp, balances: Vec<(Addr, &[Coin])>) {
    for (addr, coins) in balances {
        app.send_tokens(Addr::unchecked("bank"), addr, coins)
            .unwrap();
    }
}

pub struct MockAndromeda<'a> {
    pub admin_address: Addr,
    pub kernel: MockKernel,
    pub adodb: MockADODB,
    pub economics: MockEconomics,
    pub vfs: MockVFS,
    pub wallets: HashMap<String, Addr>,
    pub router: &'a MockApp,
}

impl<'a> MockAndromeda<'a> {
    pub fn new(app: &'a mut MockApp, admin_address: &Addr) -> MockAndromeda<'a> {
        let admin_address = app.api().addr_make(admin_address.as_str());
        // Store contract codes
        let adodb_code_id = app.store_code(mock_andromeda_adodb());
        let kernel_code_id = app.store_code(mock_andromeda_kernel());
        let vfs_code_id = app.store_code(mock_andromeda_vfs());
        let economics_code_id = app.store_code(mock_andromeda_economics());

        // Init Kernel
        let kernel = MockKernel::instantiate(
            app,
            kernel_code_id,
            admin_address.clone(),
            Some(admin_address.to_string()),
            None,
        );

        // Init ADO DB
        let adodb = MockADODB::instantiate(
            app,
            adodb_code_id,
            admin_address.clone(),
            None,
            kernel.addr().to_string(),
        );

        //Init Economics
        let economics = MockEconomics::instantiate(
            app,
            economics_code_id,
            admin_address.clone(),
            None,
            kernel.addr().to_string(),
        );

        // Init VFS
        let vfs = MockVFS::instantiate(
            app,
            vfs_code_id,
            admin_address.clone(),
            None,
            kernel.addr().to_string(),
        );

        // Add Code IDs
        adodb
            .execute_publish(
                app,
                admin_address.clone(),
                adodb_code_id,
                "adodb",
                "0.1.0",
                None,
                None,
            )
            .unwrap();
        adodb
            .execute_publish(
                app,
                admin_address.clone(),
                vfs_code_id,
                "vfs",
                "0.1.0",
                None,
                None,
            )
            .unwrap();
        adodb
            .execute_publish(
                app,
                admin_address.clone(),
                kernel_code_id,
                "kernel",
                "0.1.0",
                None,
                None,
            )
            .unwrap();
        kernel
            .execute_store_key_address(app, admin_address.clone(), "adodb", adodb.addr().clone())
            .unwrap();
        kernel
            .execute_store_key_address(app, admin_address.clone(), "vfs", vfs.addr().clone())
            .unwrap();
        kernel
            .execute_store_key_address(
                app,
                admin_address.clone(),
                "economics",
                economics.addr().clone(),
            )
            .unwrap();

        MockAndromeda {
            admin_address,
            kernel,
            adodb,
            economics,
            vfs,
            wallets: HashMap::new(),
            router: app,
        }
    }

    pub fn router(&self) -> &MockApp {
        &self.router
    }

    /// Stores a given Code ID under the given key in the ADO DB contract
    pub fn store_code_id(&mut self, key: &str, code_id: u64) {
        let mut router = self.router();
        self.adodb
            .execute_publish(
                &mut router,
                self.admin_address.clone(),
                code_id,
                key,
                "0.1.0",
                Some(self.admin_address.to_string()),
                None,
            )
            .unwrap();
    }

    /// Gets the Code ID for a given key from the ADO DB contract
    pub fn get_code_id(&self, app: &mut MockApp, key: impl Into<String>) -> u64 {
        self.adodb.query_code_id(app, key)
    }

    pub fn add_wallet(&mut self, name: &str) {
        let addr = self.router.api().addr_make(name);
        self.wallets
            .entry(name.to_string())
            .and_modify(|_| {
                panic!("Wallet already exists");
            })
            .or_insert(addr);
    }
}
