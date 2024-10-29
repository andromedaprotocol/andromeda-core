use andromeda_data_storage::form::{ExecuteMsg, InstantiateMsg, QueryMsg};
use andromeda_data_storage::form::{
    FormConfig, GetAllSubmissionsResponse, GetFormStatusResponse, GetSchemaResponse,
    GetSubmissionResponse,
};
use andromeda_std::{
    amp::AndrAddr, error::ContractError, testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Deps, DepsMut, MessageInfo, OwnedDeps, Response, Timestamp,
};

use crate::contract::{execute, instantiate, query};
use crate::testing::mock_querier::{mock_dependencies_custom, WasmMockQuerier};

pub type MockDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

pub fn valid_initialization(
    schema_ado_address: AndrAddr,
    authorized_addresses_for_submission: Option<Vec<AndrAddr>>,
    form_config: FormConfig,
    custom_key_for_notifications: Option<String>,
    timestamp_nanos: u64,
) -> (MockDeps, MessageInfo, Response) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        schema_ado_address,
        authorized_addresses_for_submission,
        form_config,
        custom_key_for_notifications,
    };
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(timestamp_nanos);
    let response = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();

    (deps, info.clone(), response)
}

pub fn invalid_initialization(
    schema_ado_address: AndrAddr,
    authorized_addresses_for_submission: Option<Vec<AndrAddr>>,
    form_config: FormConfig,
    custom_key_for_notifications: Option<String>,
    timestamp_nanos: u64,
) -> (MockDeps, MessageInfo, ContractError) {
    let mut deps = mock_dependencies_custom(&[]);
    let info = mock_info("creator", &[]);
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
        schema_ado_address,
        authorized_addresses_for_submission,
        form_config,
        custom_key_for_notifications,
    };
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(timestamp_nanos);
    let err = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap_err();

    (deps, info.clone(), err)
}

pub fn submit_form(
    deps: DepsMut<'_>,
    sender: &str,
    data: String,
    timestamp_nanos: u64,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::SubmitForm { data };
    let info = mock_info(sender, &[]);
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(timestamp_nanos);
    execute(deps, env, info, msg)
}

pub fn delete_submission(
    deps: DepsMut<'_>,
    sender: &str,
    submission_id: u64,
    wallet_address: AndrAddr,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::DeleteSubmission {
        submission_id,
        wallet_address,
    };
    let info = mock_info(sender, &[]);
    let env = mock_env();
    execute(deps, env, info, msg)
}

pub fn edit_submission(
    deps: DepsMut<'_>,
    sender: &str,
    submission_id: u64,
    wallet_address: AndrAddr,
    data: String,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::EditSubmission {
        submission_id,
        wallet_address,
        data,
    };
    let info = mock_info(sender, &[]);
    let env = mock_env();
    execute(deps, env, info, msg)
}

pub fn open_form(
    deps: DepsMut<'_>,
    sender: &str,
    timestamp_nanos: u64,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::OpenForm {};
    let info = mock_info(sender, &[]);
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(timestamp_nanos);
    execute(deps, env, info, msg)
}

pub fn close_form(
    deps: DepsMut<'_>,
    sender: &str,
    timestamp_nanos: u64,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::CloseForm {};
    let info = mock_info(sender, &[]);
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(timestamp_nanos);
    execute(deps, env, info, msg)
}

pub fn query_schema(deps: Deps) -> Result<GetSchemaResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetSchema {});
    match res {
        Ok(res) => Ok(from_json(res)?),
        Err(err) => Err(err),
    }
}

pub fn query_form_status(
    deps: Deps,
    timestamp_nanos: u64,
) -> Result<GetFormStatusResponse, ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(timestamp_nanos);
    let res = query(deps, env, QueryMsg::GetFormStatus {});
    match res {
        Ok(res) => Ok(from_json(res)?),
        Err(err) => Err(err),
    }
}

pub fn query_all_submissions(deps: Deps) -> Result<GetAllSubmissionsResponse, ContractError> {
    let res = query(deps, mock_env(), QueryMsg::GetAllSubmissions {});
    match res {
        Ok(res) => Ok(from_json(res)?),
        Err(err) => Err(err),
    }
}

pub fn query_submission(
    deps: Deps,
    submission_id: u64,
    wallet_address: AndrAddr,
) -> Result<GetSubmissionResponse, ContractError> {
    let res = query(
        deps,
        mock_env(),
        QueryMsg::GetSubmission {
            submission_id,
            wallet_address,
        },
    );
    match res {
        Ok(res) => Ok(from_json(res)?),
        Err(err) => Err(err),
    }
}
