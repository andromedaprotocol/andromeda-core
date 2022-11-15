#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use andromeda_adodb::mock::{
    mock_adodb_instantiate_msg, mock_andromeda_adodb, mock_get_code_id_msg, mock_store_code_id_msg,
};
use andromeda_primitive::mock::{
    mock_andromeda_primitive, mock_primitive_instantiate_msg, mock_store_address_msgs,
};
use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};

pub struct MockAndromeda {
    pub admin_address: Addr,
    pub adodb_address: Addr,
    pub registry_address: Addr,
}

impl MockAndromeda {
    pub fn new(app: &mut App, admin_address: &Addr) -> MockAndromeda {
        // Store contract codes
        let adodb_code_id = app.store_code(mock_andromeda_adodb());
        let primitive_code_id = app.store_code(mock_andromeda_primitive());

        // Init ADO DB
        let adodb_init_msg = mock_adodb_instantiate_msg();
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

        // Init Registry
        let registry_init_msg = mock_primitive_instantiate_msg();
        let registry_address = app
            .instantiate_contract(
                primitive_code_id,
                admin_address.clone(),
                &registry_init_msg,
                &[],
                "Registry",
                Some(admin_address.to_string()),
            )
            .unwrap();

        // Add Code IDs
        let store_primitive_code_id_msg =
            mock_store_code_id_msg("primitive".to_string(), primitive_code_id);
        let store_adodb_code_id_msg = mock_store_code_id_msg("adodb".to_string(), adodb_code_id); //Dev Note: In future change this to "adodb" for the key
        app.execute_contract(
            admin_address.clone(),
            adodb_address.clone(),
            &store_primitive_code_id_msg,
            &[],
        )
        .unwrap();
        app.execute_contract(
            admin_address.clone(),
            adodb_address.clone(),
            &store_adodb_code_id_msg,
            &[],
        )
        .unwrap();

        // Store ADO DB address
        let store_adodb_addr_msg =
            mock_store_address_msgs("adodb".to_string(), adodb_address.to_string());
        app.execute_contract(
            admin_address.clone(),
            registry_address.clone(),
            &store_adodb_addr_msg,
            &[],
        )
        .unwrap();

        MockAndromeda {
            adodb_address,
            registry_address,
            admin_address: admin_address.clone(),
        }
    }

    /// Stores a given Code ID under the given key in the ADO DB contract
    pub fn store_code_id(&self, app: &mut App, key: &str, code_id: u64) {
        let msg = mock_store_code_id_msg(key.to_string(), code_id);

        app.execute_contract(
            self.admin_address.clone(),
            self.adodb_address.clone(),
            &msg,
            &[],
        )
        .unwrap();
    }

    /// Gets the Code ID for a given key from the ADO DB contract
    pub fn get_code_id(&self, app: &mut App, key: String) -> u64 {
        let msg = mock_get_code_id_msg(key);

        app.wrap()
            .query_wasm_smart(self.adodb_address.clone(), &msg)
            .unwrap()
    }
}
