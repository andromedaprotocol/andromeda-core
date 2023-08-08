#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use andromeda_adodb::mock::{
    mock_adodb_instantiate_msg, mock_andromeda_adodb, mock_get_code_id_msg, mock_publish,
    mock_store_code_id_msg,
};
use andromeda_economics::mock::{mock_andromeda_economics, mock_economics_instantiate_msg};
use andromeda_kernel::mock::{
    mock_andromeda_kernel, mock_get_key_address, mock_kernel_instantiate_message,
    mock_upsert_key_address,
};
use andromeda_vfs::mock::{
    mock_add_path, mock_andromeda_vfs, mock_register_user, mock_resolve_path_query,
    mock_vfs_instantiate_message,
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, Contract, Executor};

pub const ADMIN_USERNAME: &str = "am";

pub struct MockAndromeda {
    pub admin_address: Addr,
    pub adodb_address: Addr,
    pub kernel_address: Addr,
}

impl MockAndromeda {
    pub fn new(app: &mut App, admin_address: &Addr) -> MockAndromeda {
        // Store contract codes
        let adodb_code_id = app.store_code(mock_andromeda_adodb());
        let kernel_code_id = app.store_code(mock_andromeda_kernel());
        let vfs_code_id = app.store_code(mock_andromeda_vfs());
        let economics_code_id = app.store_code(mock_andromeda_economics());

        // Init Kernel
        let kernel_init_msg = mock_kernel_instantiate_message(None);
        let kernel_address = app
            .instantiate_contract(
                kernel_code_id,
                admin_address.clone(),
                &kernel_init_msg,
                &[],
                "Kernel",
                Some(admin_address.to_string()),
            )
            .unwrap();

        // Init ADO DB
        let adodb_init_msg = mock_adodb_instantiate_msg(kernel_address.clone(), None);
        let adodb_address = app
            .instantiate_contract(
                adodb_code_id,
                admin_address.clone(),
                &adodb_init_msg,
                &[],
                "ADO DB",
                Some(admin_address.to_string()),
            )
            .unwrap();

        //Init Economics
        let economics_init_msg = mock_economics_instantiate_msg(kernel_address.clone(), None);
        let economics_address = app
            .instantiate_contract(
                economics_code_id,
                admin_address.clone(),
                &economics_init_msg,
                &[],
                "Andr Economics",
                Some(admin_address.to_string()),
            )
            .unwrap();

        // Init VFS
        let vfs_init_msg = mock_vfs_instantiate_message(kernel_address.clone(), None);
        let vfs_address = app
            .instantiate_contract(
                vfs_code_id,
                admin_address.clone(),
                &vfs_init_msg,
                &[],
                "VFS",
                Some(admin_address.to_string()),
            )
            .unwrap();

        // Add Code IDs
        let store_adodb_code_id_msg = mock_store_code_id_msg("adodb".to_string(), adodb_code_id); //Dev Note: In future change this to "adodb" for the key
        let store_kernel_code_id_msg = mock_store_code_id_msg("kernel".to_string(), kernel_code_id);
        let store_economics_code_id_msg =
            mock_store_code_id_msg("economics".to_string(), economics_code_id);
        app.execute_contract(
            admin_address.clone(),
            adodb_address.clone(),
            &store_adodb_code_id_msg,
            &[],
        )
        .unwrap();
        app.execute_contract(
            admin_address.clone(),
            adodb_address.clone(),
            &store_kernel_code_id_msg,
            &[],
        )
        .unwrap();
        app.execute_contract(
            admin_address.clone(),
            adodb_address.clone(),
            &store_economics_code_id_msg,
            &[],
        )
        .unwrap();

        let mock_andr = MockAndromeda {
            adodb_address: adodb_address.clone(),
            admin_address: admin_address.clone(),
            kernel_address,
        };

        mock_andr.register_kernel_key_address(app, "adodb", adodb_address);
        mock_andr.register_kernel_key_address(app, "vfs", vfs_address);
        mock_andr.register_kernel_key_address(app, "economics", economics_address);
        mock_andr.register_user(app, admin_address.clone(), ADMIN_USERNAME);

        mock_andr
    }

    /// Stores a given Code ID under the given key in the ADO DB contract
    pub fn store_code_id(&self, app: &mut App, key: &str, code_id: u64) {
        let msg = mock_publish(code_id, key, "0.1.0", None, None);

        app.execute_contract(
            self.admin_address.clone(),
            self.adodb_address.clone(),
            &msg,
            &[],
        )
        .unwrap();
    }

    pub fn store_ado(
        &self,
        app: &mut App,
        contract: Box<dyn Contract<Empty>>,
        ado_type: impl Into<String>,
    ) {
        let code_id = app.store_code(contract);
        self.store_code_id(app, ado_type.into().as_str(), code_id);
    }

    /// Gets the Code ID for a given key from the ADO DB contract
    pub fn get_code_id(&self, app: &mut App, key: impl Into<String>) -> u64 {
        let msg = mock_get_code_id_msg(key.into());

        app.wrap()
            .query_wasm_smart(self.adodb_address.clone(), &msg)
            .unwrap()
    }

    /// Registers a key address for the kernel
    pub fn register_kernel_key_address(
        &self,
        app: &mut App,
        key: impl Into<String>,
        address: Addr,
    ) {
        let msg = mock_upsert_key_address(key, address);
        app.execute_contract(
            self.admin_address.clone(),
            self.kernel_address.clone(),
            &msg,
            &[],
        )
        .unwrap();
    }

    /// Registers a user on the VFS
    pub fn register_user(&self, app: &mut App, sender: Addr, username: impl Into<String>) {
        let vfs_address_query = mock_get_key_address("vfs");
        let vfs_address: Addr = app
            .wrap()
            .query_wasm_smart(self.kernel_address.clone(), &vfs_address_query)
            .unwrap();

        let register_msg = mock_register_user(username);

        app.execute_contract(sender, vfs_address, &register_msg, &[])
            .unwrap();
    }

    /// Adds a path to resolve to the VFS
    pub fn vfs_add_path(
        &self,
        app: &mut App,
        sender: Addr,
        name: impl Into<String>,
        address: Addr,
    ) {
        let vfs_address_query = mock_get_key_address("vfs");
        let vfs_address: Addr = app
            .wrap()
            .query_wasm_smart(self.kernel_address.clone(), &vfs_address_query)
            .unwrap();

        let register_msg = mock_add_path(name, address);
        app.execute_contract(sender, vfs_address, &register_msg, &[])
            .unwrap();
    }

    pub fn vfs_resolve_path(&self, app: &mut App, path: impl Into<String>) -> Addr {
        let vfs_address_query = mock_get_key_address("vfs");
        let vfs_address: Addr = app
            .wrap()
            .query_wasm_smart(self.kernel_address.clone(), &vfs_address_query)
            .unwrap();

        let query = mock_resolve_path_query(path);
        app.wrap().query_wasm_smart(vfs_address, &query).unwrap()
    }
}
