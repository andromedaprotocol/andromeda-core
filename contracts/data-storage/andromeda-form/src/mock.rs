#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_data_storage::form::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_data_storage::form::{
    FormConfig, GetAllSubmissionsResponse, GetFormStatusResponse, GetSchemaResponse,
    GetSubmissionResponse,
};
use andromeda_std::amp::AndrAddr;
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub struct MockForm(Addr);
mock_ado!(MockForm, ExecuteMsg, QueryMsg);

impl MockForm {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        schema_ado_address: AndrAddr,
        authorized_addresses_for_submission: Option<Vec<AndrAddr>>,
        form_config: FormConfig,
        custom_key_for_notifications: Option<String>,
    ) -> MockForm {
        let msg = mock_form_instantiate_msg(
            kernel_address,
            owner,
            schema_ado_address,
            authorized_addresses_for_submission,
            form_config,
            custom_key_for_notifications,
        );
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Form Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockForm(Addr::unchecked(addr))
    }

    pub fn execute_submit_form(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
        data: String,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::SubmitForm { data };

        // Conditionally build the funds vector
        let funds_vec = match funds {
            Some(funds) => vec![funds],
            None => vec![],
        };

        // Call the method once
        app.execute_contract(sender, self.addr().clone(), &msg, &funds_vec)
    }

    pub fn execute_delete_submission(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
        submission_id: u64,
        wallet_address: AndrAddr,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::DeleteSubmission {
            submission_id,
            wallet_address,
        };

        // Conditionally build the funds vector
        let funds_vec = match funds {
            Some(funds) => vec![funds],
            None => vec![],
        };

        // Call the method once
        app.execute_contract(sender, self.addr().clone(), &msg, &funds_vec)
    }

    pub fn execute_edit_submission(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
        submission_id: u64,
        wallet_address: AndrAddr,
        data: String,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::EditSubmission {
            submission_id,
            wallet_address,
            data,
        };

        // Conditionally build the funds vector
        let funds_vec = match funds {
            Some(funds) => vec![funds],
            None => vec![],
        };

        // Call the method once
        app.execute_contract(sender, self.addr().clone(), &msg, &funds_vec)
    }

    pub fn execute_open_form(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::OpenForm {};

        // Conditionally build the funds vector
        let funds_vec = match funds {
            Some(funds) => vec![funds],
            None => vec![],
        };

        // Call the method once
        app.execute_contract(sender, self.addr().clone(), &msg, &funds_vec)
    }

    pub fn execute_close_form(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds: Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::CloseForm {};

        // Conditionally build the funds vector
        let funds_vec = match funds {
            Some(funds) => vec![funds],
            None => vec![],
        };

        // Call the method once
        app.execute_contract(sender, self.addr().clone(), &msg, &funds_vec)
    }

    pub fn query_schema(&self, app: &mut MockApp) -> GetSchemaResponse {
        let msg = QueryMsg::GetSchema {};
        let res: GetSchemaResponse = self.query(app, msg);
        res
    }

    pub fn query_form_status(&self, app: &mut MockApp) -> GetFormStatusResponse {
        let msg = QueryMsg::GetFormStatus {};
        let res: GetFormStatusResponse = self.query(app, msg);
        res
    }

    pub fn query_all_submissions(&self, app: &mut MockApp) -> GetAllSubmissionsResponse {
        let msg = QueryMsg::GetAllSubmissions {};
        let res: GetAllSubmissionsResponse = self.query(app, msg);
        res
    }

    pub fn query_submission(
        &self,
        app: &mut MockApp,
        submission_id: u64,
        wallet_address: AndrAddr,
    ) -> GetSubmissionResponse {
        let msg = QueryMsg::GetSubmission {
            submission_id,
            wallet_address,
        };
        let res: GetSubmissionResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_form() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_form_instantiate_msg(
    kernel_address: impl Into<String>,
    owner: Option<String>,
    schema_ado_address: AndrAddr,
    authorized_addresses_for_submission: Option<Vec<AndrAddr>>,
    form_config: FormConfig,
    custom_key_for_notifications: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
        schema_ado_address,
        authorized_addresses_for_submission,
        form_config,
        custom_key_for_notifications,
    }
}
