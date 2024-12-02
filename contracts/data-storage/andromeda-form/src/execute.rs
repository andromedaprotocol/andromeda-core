use andromeda_data_storage::form::{ExecuteMsg, SubmissionInfo};
use andromeda_modules::schema::{QueryMsg as SchemaQueryMsg, ValidateDataResponse};
use andromeda_std::{
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{
        actions::call_action, context::ExecuteContext, encode_binary,
        expiration::get_and_validate_start_time, Milliseconds,
    },
    error::ContractError,
};
use cosmwasm_std::{ensure, QueryRequest, Response, Uint64, WasmQuery};
use cw_utils::{nonpayable, Expiration};

use crate::{
    contract::SUBMIT_FORM_ACTION,
    state::{
        submissions, ALLOW_EDIT_SUBMISSION, ALLOW_MULTIPLE_SUBMISSIONS, END_TIME,
        SCHEMA_ADO_ADDRESS, START_TIME, SUBMISSION_ID,
    },
};

const MAX_LIMIT: u64 = 100u64;

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;

    let res = match msg {
        ExecuteMsg::SubmitForm { data } => execute_submit_form(ctx, data),
        ExecuteMsg::DeleteSubmission {
            submission_id,
            wallet_address,
        } => execute_delete_submission(ctx, submission_id, wallet_address),
        ExecuteMsg::EditSubmission {
            submission_id,
            wallet_address,
            data,
        } => execute_edit_submission(ctx, submission_id, wallet_address, data),
        ExecuteMsg::OpenForm {} => execute_open_form(ctx),
        ExecuteMsg::CloseForm {} => execute_close_form(ctx),
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

pub fn execute_submit_form(
    mut ctx: ExecuteContext,
    data: String,
) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    ADOContract::default().is_permissioned(
        ctx.deps.branch(),
        ctx.env.clone(),
        SUBMIT_FORM_ACTION,
        sender.clone(),
    )?;

    let (expiration, _) = get_and_validate_start_time(&ctx.env, None)?;
    let current_time = milliseconds_from_expiration(expiration)?;
    let saved_start_time = START_TIME.load(ctx.deps.storage)?;
    let saved_end_time = END_TIME.load(ctx.deps.storage)?;
    // validate if the Form is opened
    validate_form_is_opened(current_time, saved_start_time, saved_end_time)?;

    let schema_ado_address = SCHEMA_ADO_ADDRESS.load(ctx.deps.storage)?;
    let data_to_validate = data.clone();
    let validate_res: ValidateDataResponse =
        ctx.deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: schema_ado_address
                    .get_raw_address(&ctx.deps.as_ref())?
                    .into_string(),
                msg: encode_binary(&SchemaQueryMsg::ValidateData {
                    data: data_to_validate,
                })?,
            }))?;

    let submission_id = SUBMISSION_ID.load(ctx.deps.storage)?;
    let new_id = submission_id.checked_add(Uint64::one())?;

    match validate_res {
        ValidateDataResponse::Valid => {
            let allow_multiple_submissions = ALLOW_MULTIPLE_SUBMISSIONS.load(ctx.deps.storage)?;
            match allow_multiple_submissions {
                true => {
                    submissions().save(
                        ctx.deps.storage,
                        &(new_id.u64(), sender.clone()),
                        &SubmissionInfo {
                            submission_id: new_id.u64(),
                            wallet_address: sender.clone(),
                            data,
                        },
                    )?;
                    SUBMISSION_ID.save(ctx.deps.storage, &new_id)?;
                }
                false => {
                    let submissions_by_address: Vec<SubmissionInfo> = submissions()
                        .idx
                        .wallet_address
                        .prefix(sender.clone())
                        .range(ctx.deps.storage, None, None, cosmwasm_std::Order::Ascending)
                        .take(MAX_LIMIT as usize)
                        .map(|r| r.unwrap().1)
                        .collect();

                    if submissions_by_address.is_empty() {
                        submissions().save(
                            ctx.deps.storage,
                            &(new_id.u64(), sender.clone()),
                            &SubmissionInfo {
                                submission_id: new_id.u64(),
                                wallet_address: sender.clone(),
                                data,
                            },
                        )?;
                        SUBMISSION_ID.save(ctx.deps.storage, &new_id)?;
                    } else {
                        return Err(ContractError::CustomError {
                            msg: "Multiple submissions are not allowed".to_string(),
                        });
                    }
                }
            }
        }
        ValidateDataResponse::Invalid { msg } => return Err(ContractError::CustomError { msg }),
    }

    let response = Response::new()
        .add_attribute("method", "submit_form")
        .add_attribute("submission_id", new_id)
        .add_attribute("sender", sender.clone());

    Ok(response)
}

pub fn execute_delete_submission(
    ctx: ExecuteContext,
    submission_id: u64,
    wallet_address: AndrAddr,
) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );

    let address = wallet_address.get_raw_address(&ctx.deps.as_ref())?;
    submissions()
        .load(ctx.deps.storage, &(submission_id, address.clone()))
        .map_err(|_| ContractError::CustomError {
            msg: format!(
                "Submission does not exist - Submission_id {:?}, Wallet_address {:?}",
                submission_id, wallet_address
            ),
        })?;
    submissions().remove(ctx.deps.storage, &(submission_id, address.clone()))?;

    let response = Response::new()
        .add_attribute("method", "delete_submission")
        .add_attribute("submission_id", Uint64::from(submission_id))
        .add_attribute("sender", sender);

    Ok(response)
}

pub fn execute_edit_submission(
    ctx: ExecuteContext,
    submission_id: u64,
    wallet_address: AndrAddr,
    data: String,
) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    let allow_edit_submission = ALLOW_EDIT_SUBMISSION.load(ctx.deps.storage)?;
    ensure!(
        allow_edit_submission,
        ContractError::CustomError {
            msg: "Edit submission is not allowed".to_string(),
        }
    );
    let (expiration, _) = get_and_validate_start_time(&ctx.env, None)?;
    let current_time = milliseconds_from_expiration(expiration)?;
    let saved_start_time = START_TIME.load(ctx.deps.storage)?;
    let saved_end_time = END_TIME.load(ctx.deps.storage)?;
    // validate if the Form is opened
    validate_form_is_opened(current_time, saved_start_time, saved_end_time)?;

    let schema_ado_address = SCHEMA_ADO_ADDRESS.load(ctx.deps.storage)?;
    let data_to_validate = data.clone();
    let validate_res: ValidateDataResponse =
        ctx.deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: schema_ado_address
                    .get_raw_address(&ctx.deps.as_ref())?
                    .into_string(),
                msg: encode_binary(&SchemaQueryMsg::ValidateData {
                    data: data_to_validate,
                })?,
            }))?;

    let wallet_address = wallet_address.get_raw_address(&ctx.deps.as_ref())?;

    if submissions()
        .may_load(ctx.deps.storage, &(submission_id, wallet_address.clone()))?
        .is_some()
    {
        ensure!(
            wallet_address.clone() == sender.clone(),
            ContractError::Unauthorized {}
        );

        match validate_res {
            ValidateDataResponse::Valid => {
                submissions().save(
                    ctx.deps.storage,
                    &(submission_id, wallet_address),
                    &SubmissionInfo {
                        submission_id,
                        wallet_address: sender.clone(),
                        data: data.clone(),
                    },
                )?;
            }
            ValidateDataResponse::Invalid { msg } => {
                return Err(ContractError::CustomError { msg })
            }
        }
    } else {
        return Err(ContractError::CustomError {
            msg: format!(
                "Submission is not existed - Submission_id {:?}, Wallet_address {:?}",
                submission_id, wallet_address
            ),
        });
    }

    let response = Response::new()
        .add_attribute("method", "edit_submission")
        .add_attribute("submission_id", Uint64::from(submission_id))
        .add_attribute("sender", sender);

    Ok(response)
}

pub fn execute_open_form(ctx: ExecuteContext) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );

    let (start_expiration, _) = get_and_validate_start_time(&ctx.env, None)?;
    let start_time = milliseconds_from_expiration(start_expiration)?;

    let saved_start_time = START_TIME.load(ctx.deps.storage)?;
    let saved_end_time = END_TIME.load(ctx.deps.storage)?;
    match saved_start_time {
        Some(saved_start_time) => match saved_end_time {
            Some(saved_end_time) => {
                if saved_start_time.gt(&start_time) {
                    START_TIME.save(ctx.deps.storage, &Some(start_time))?;
                } else if saved_end_time.gt(&start_time) {
                    return Err(ContractError::CustomError {
                        msg: format!("Already opened. Opened time {:?}", saved_start_time),
                    });
                } else {
                    START_TIME.save(ctx.deps.storage, &Some(start_time))?;
                    END_TIME.save(ctx.deps.storage, &None)?;
                }
            }
            None => {
                if saved_start_time.gt(&start_time) {
                    START_TIME.save(ctx.deps.storage, &Some(start_time))?;
                } else {
                    return Err(ContractError::CustomError {
                        msg: format!("Already opened. Opened time {:?}", saved_start_time),
                    });
                }
            }
        },
        None => {
            START_TIME.save(ctx.deps.storage, &Some(start_time))?;
            if let Some(saved_end_time) = saved_end_time {
                if start_time.gt(&saved_end_time) {
                    END_TIME.save(ctx.deps.storage, &None)?;
                }
            }
        }
    }

    let response = Response::new()
        .add_attribute("method", "open_form")
        .add_attribute("sender", sender);

    Ok(response)
}

pub fn execute_close_form(ctx: ExecuteContext) -> Result<Response, ContractError> {
    nonpayable(&ctx.info)?;
    let sender = ctx.info.sender;
    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );

    let (end_expiration, _) = get_and_validate_start_time(&ctx.env, None)?;
    let end_time = milliseconds_from_expiration(end_expiration)?;

    let saved_start_time = START_TIME.load(ctx.deps.storage)?;
    let saved_end_time = END_TIME.load(ctx.deps.storage)?;
    match saved_end_time {
        Some(saved_end_time) => match saved_start_time {
            Some(saved_start_time) => {
                if saved_start_time.gt(&end_time) {
                    return Err(ContractError::CustomError {
                        msg: format!("Not opened yet. Will be opend at {:?}", saved_start_time),
                    });
                } else if saved_end_time.gt(&end_time) {
                    END_TIME.save(ctx.deps.storage, &Some(end_time))?;
                } else {
                    return Err(ContractError::CustomError {
                        msg: format!("Already closed. Closed at {:?}", saved_end_time),
                    });
                }
            }
            None => {
                return Err(ContractError::CustomError {
                    msg: "Not opened yet".to_string(),
                })
            }
        },
        None => match saved_start_time {
            Some(saved_start_time) => {
                if saved_start_time.gt(&end_time) {
                    return Err(ContractError::CustomError {
                        msg: format!("Not opened yet. Will be opend at {:?}", saved_start_time),
                    });
                } else {
                    END_TIME.save(ctx.deps.storage, &Some(end_time))?;
                }
            }
            None => {
                return Err(ContractError::CustomError {
                    msg: "Not opened yet".to_string(),
                })
            }
        },
    }

    let response = Response::new()
        .add_attribute("method", "close_form")
        .add_attribute("sender", sender);

    Ok(response)
}

pub fn milliseconds_from_expiration(expiration: Expiration) -> Result<Milliseconds, ContractError> {
    match expiration {
        Expiration::AtTime(time) => Ok(Milliseconds::from_nanos(time.nanos())),
        _ => Err(ContractError::CustomError {
            msg: "Not supported expiration enum".to_string(),
        }),
    }
}

pub fn validate_form_is_opened(
    current_time: Milliseconds,
    saved_start_time: Option<Milliseconds>,
    saved_end_time: Option<Milliseconds>,
) -> Result<(), ContractError> {
    match saved_start_time {
        Some(saved_start_time) => {
            if saved_start_time.gt(&current_time) {
                return Err(ContractError::CustomError {
                    msg: format!("Not opened yet. Will be opend at {:?}", saved_start_time),
                });
            }
            if let Some(saved_end_time) = saved_end_time {
                if current_time.gt(&saved_end_time) {
                    return Err(ContractError::CustomError {
                        msg: format!("Already closed. Closed at {:?}", saved_end_time),
                    });
                }
            }
            Ok(())
        }
        None => Err(ContractError::CustomError {
            msg: "Not opened yet. Start time is not set".to_string(),
        }),
    }
}
