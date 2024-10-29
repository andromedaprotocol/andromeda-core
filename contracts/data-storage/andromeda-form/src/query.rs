use andromeda_data_storage::form::{
    GetAllSubmissionsResponse, GetFormStatusResponse, GetSchemaResponse, GetSubmissionResponse,
    SubmissionInfo,
};
use andromeda_modules::schema::{GetSchemaResponse as SchemaResponse, QueryMsg as SchemaQueryMsg};
use andromeda_std::{
    amp::AndrAddr,
    common::{encode_binary, expiration::get_and_validate_start_time},
    error::ContractError,
};
use cosmwasm_std::{Deps, Env, Storage};
use cosmwasm_std::{QueryRequest, WasmQuery};

use crate::execute::{milliseconds_from_expiration, validate_form_is_opened};
use crate::state::{submissions, END_TIME, SCHEMA_ADO_ADDRESS, START_TIME};

pub fn get_schema(deps: Deps) -> Result<GetSchemaResponse, ContractError> {
    let schema_ado_address = SCHEMA_ADO_ADDRESS.load(deps.storage)?;
    let res: SchemaResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: schema_ado_address.get_raw_address(&deps)?.into_string(),
        msg: encode_binary(&SchemaQueryMsg::GetSchema {})?,
    }))?;
    let schema = res.schema;
    Ok(GetSchemaResponse { schema })
}

pub fn get_form_status(
    storage: &dyn Storage,
    env: Env,
) -> Result<GetFormStatusResponse, ContractError> {
    let (expiration, _) = get_and_validate_start_time(&env, None)?;
    let current_time = milliseconds_from_expiration(expiration)?;
    let saved_start_time = START_TIME.load(storage)?;
    let saved_end_time = END_TIME.load(storage)?;
    // validate if the Form is opened
    let res_validation = validate_form_is_opened(current_time, saved_start_time, saved_end_time);
    if res_validation.is_ok() {
        Ok(GetFormStatusResponse::Opened)
    } else {
        Ok(GetFormStatusResponse::Closed)
    }
}

pub fn get_all_submissions(
    storage: &dyn Storage,
) -> Result<GetAllSubmissionsResponse, ContractError> {
    let all_submissions: Vec<SubmissionInfo> = submissions()
        .idx
        .submission_id
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|r| r.unwrap().1) // Extract the `SubmissionInfo` from the result
        .collect();
    Ok(GetAllSubmissionsResponse { all_submissions })
}

pub fn get_submission(
    deps: Deps,
    submission_id: u64,
    wallet_address: AndrAddr,
) -> Result<GetSubmissionResponse, ContractError> {
    let wallet_address = wallet_address.get_raw_address(&deps)?;
    let submission =
        submissions().may_load(deps.storage, &(submission_id, wallet_address.clone()))?;
    Ok(GetSubmissionResponse { submission })
}
