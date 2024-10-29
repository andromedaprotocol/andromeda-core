use andromeda_std::{amp::AndrAddr, common::expiration::Expiry};
use andromeda_std::{andr_exec, andr_instantiate, andr_query};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[andr_instantiate]
#[cw_serde]
pub struct InstantiateMsg {
    pub schema_ado_address: AndrAddr, // Address of the schema ADO
    pub authorized_addresses_for_submission: Option<Vec<AndrAddr>>,
    pub form_config: FormConfig,
    pub custom_key_for_notifications: Option<String>,
}

#[cw_serde]
pub struct FormConfig {
    pub start_time: Option<Expiry>,       // Optional start time for form
    pub end_time: Option<Expiry>,         // Optional end time for form
    pub allow_multiple_submissions: bool, // Whether multiple submissions are allowed
    pub allow_edit_submission: bool,      // Whether users can edit their submission
}

#[andr_exec]
#[cw_serde]
pub enum ExecuteMsg {
    SubmitForm {
        data: String,
    },
    DeleteSubmission {
        submission_id: u64,
        wallet_address: AndrAddr,
    },
    EditSubmission {
        submission_id: u64,
        wallet_address: AndrAddr,
        data: String,
    },
    OpenForm {},
    CloseForm {},
}

#[andr_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetSchemaResponse)]
    GetSchema {},
    #[returns(GetAllSubmissionsResponse)]
    GetAllSubmissions {},
    #[returns(GetSubmissionResponse)]
    GetSubmission {
        submission_id: u64,
        wallet_address: AndrAddr,
    },
    #[returns(GetFormStatusResponse)]
    GetFormStatus {},
}

#[cw_serde]
pub struct GetSchemaResponse {
    pub schema: String,
}

#[cw_serde]
pub struct GetAllSubmissionsResponse {
    pub all_submissions: Vec<SubmissionInfo>,
}

#[cw_serde]
pub struct GetSubmissionResponse {
    pub submission: Option<SubmissionInfo>,
}

#[cw_serde]
pub struct SubmissionInfo {
    pub submission_id: u64,
    pub wallet_address: Addr,
    pub data: String,
}

#[cw_serde]
pub enum GetFormStatusResponse {
    Opened,
    Closed,
}
