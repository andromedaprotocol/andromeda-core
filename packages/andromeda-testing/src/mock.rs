use andromeda_adodb::mock::mock_andromeda_adodb;
use andromeda_economics::mock::mock_andromeda_economics;
use andromeda_kernel::mock::mock_andromeda_kernel;
use andromeda_vfs::mock::mock_andromeda_vfs;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, Contract};

use crate::{mock_contract::MockContract, MockADODB, MockEconomics, MockKernel, MockVFS};

pub const ADMIN_USERNAME: &str = "am";

pub struct MockAndromeda {
    pub admin_address: Addr,
    pub kernel: MockKernel,
    pub adodb: MockADODB,
    pub economics: MockEconomics,
    pub vfs: MockVFS,
}

impl MockAndromeda {
    pub fn new(app: &mut App, admin_address: &Addr) -> MockAndromeda {
        // Store contract codes
        let adodb_code_id = app.store_code(mock_andromeda_adodb());
        let kernel_code_id = app.store_code(mock_andromeda_kernel());
        let vfs_code_id = app.store_code(mock_andromeda_vfs());
        let economics_code_id = app.store_code(mock_andromeda_economics());

        // Init Kernel
        let kernel =
            MockKernel::instantiate(app, kernel_code_id, admin_address.clone(), None, None);

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
        vfs.execute_register_user(app, admin_address.clone(), ADMIN_USERNAME.to_string())
            .unwrap();

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
            admin_address: admin_address.clone(),
            kernel,
            adodb,
            economics,
            vfs,
        }
    }

    /// Stores a given Code ID under the given key in the ADO DB contract
    pub fn store_code_id(&self, app: &mut App, key: &str, code_id: u64) {
        self.adodb
            .execute_publish(
                app,
                self.admin_address.clone(),
                code_id,
                key,
                "0.1.0",
                Some(self.admin_address.to_string()),
                None,
            )
            .unwrap();
    }

    pub fn store_ado(
        &self,
        app: &mut App,
        contract: Box<dyn Contract<Empty>>,
        ado_type: impl Into<String>,
    ) -> u64 {
        let code_id = app.store_code(contract);
        self.store_code_id(app, ado_type.into().as_str(), code_id);
        code_id
    }

    /// Gets the Code ID for a given key from the ADO DB contract
    pub fn get_code_id(&self, app: &mut App, key: impl Into<String>) -> u64 {
        self.adodb.query_code_id(app, key)
    }
}
