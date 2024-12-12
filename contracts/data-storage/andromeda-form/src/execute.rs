use andromeda_data_storage::form::{ExecuteMsg, SubmissionInfo};
use andromeda_modules::schema::{QueryMsg as SchemaQueryMsg, ValidateDataResponse};
use andromeda_std::{
    ado_contract::ADOContract,
    amp::AndrAddr,
    common::{actions::call_action, context::ExecuteContext, encode_binary, Milliseconds},
    error::ContractError,
};
use cosmwasm_std::{ensure, Env, QueryRequest, Response, Uint64, WasmQuery};
use cw_utils::{nonpayable, Expiration};

use crate::{
    contract::SUBMIT_FORM_ACTION,
    state::{submissions, Config, CONFIG, SCHEMA_ADO_ADDRESS, SUBMISSION_ID},
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

    let config = CONFIG.load(ctx.deps.storage)?;
    validate_form_is_opened(ctx.env, config.clone())?;

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
            let allow_multiple_submissions = config.allow_multiple_submissions;
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

    let config = CONFIG.load(ctx.deps.storage)?;
    let allow_edit_submission = config.allow_edit_submission;
    ensure!(
        allow_edit_submission,
        ContractError::CustomError {
            msg: "Edit submission is not allowed".to_string(),
        }
    );
    // validate if the Form is opened
    validate_form_is_opened(ctx.env, config.clone())?;

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

    let mut config = CONFIG.load(ctx.deps.storage)?;

    let current_time = Milliseconds::from_nanos(ctx.env.block.time.nanos());
    let start_time = current_time.plus_milliseconds(Milliseconds(1));

    let saved_start_time = config.start_time;
    let saved_end_time = config.end_time;
    match saved_start_time {
        // If a start time is already configured:
        Some(saved_start_time) => match saved_end_time {
            // If both start time and end time are configured:
            Some(saved_end_time) => {
                // If the start time is in the future, update the start time to the current start_time value.
                if saved_start_time.gt(&start_time) {
                    config.start_time = Some(start_time);
                    CONFIG.save(ctx.deps.storage, &config)?;
                }
                // If the form is still open (end time is in the future), return an error as the form is already open.
                else if saved_end_time.gt(&start_time) {
                    return Err(ContractError::CustomError {
                        msg: format!("Already opened. Opened time {:?}", saved_start_time),
                    });
                }
                // Otherwise, the form was closed. Update the start time to reopen the form and clear the end time.
                else {
                    config.start_time = Some(start_time);
                    config.end_time = None;
                    CONFIG.save(ctx.deps.storage, &config)?;
                }
            }

            // If only the start time is configured (no end time):
            None => {
                // Update the start time if the saved start time is in the future.
                if saved_start_time.gt(&start_time) {
                    config.start_time = Some(start_time);
                    CONFIG.save(ctx.deps.storage, &config)?;
                }
                // Otherwise, the form is already open, return an error.
                else {
                    return Err(ContractError::CustomError {
                        msg: format!("Already opened. Opened time {:?}", saved_start_time),
                    });
                }
            }
        },
        // If no start time is configured:
        None => {
            // Set the start time to the current start_time value.
            config.start_time = Some(start_time);
            CONFIG.save(ctx.deps.storage, &config)?;

            // If an end time exists and is in the past, clear it to reopen the form.
            if let Some(saved_end_time) = saved_end_time {
                if start_time.gt(&saved_end_time) {
                    config.end_time = None;
                    CONFIG.save(ctx.deps.storage, &config)?;
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

    let current_time = Milliseconds::from_nanos(ctx.env.block.time.nanos());
    let end_time = current_time.plus_milliseconds(Milliseconds(1));

    let mut config = CONFIG.load(ctx.deps.storage)?;
    let saved_start_time = config.start_time;
    let saved_end_time = config.end_time;
    match saved_end_time {
        // If an end time is configured:
        Some(saved_end_time) => match saved_start_time {
            // If both start time and end time are configured:
            Some(saved_start_time) => {
                // If the form start time is in the future, return an error indicating the form isn't open yet.
                if saved_start_time.gt(&end_time) {
                    return Err(ContractError::CustomError {
                        msg: format!("Not opened yet. Will be opened at {:?}", saved_start_time),
                    });
                }
                // If the form is still open (end time is in the future), update the end time to the current end_time value.
                else if saved_end_time.gt(&end_time) {
                    config.end_time = Some(end_time);
                    CONFIG.save(ctx.deps.storage, &config)?;
                }
                // Otherwise, the form has already been closed. Return an error.
                else {
                    return Err(ContractError::CustomError {
                        msg: format!("Already closed. Closed at {:?}", saved_end_time),
                    });
                }
            }
            // If no start time is configured:
            None => {
                // Return an error indicating the form has not been opened yet.
                return Err(ContractError::CustomError {
                    msg: "Not opened yet".to_string(),
                });
            }
        },
        // If no end time is configured:
        None => match saved_start_time {
            // If the start time exists:
            Some(saved_start_time) => {
                // If the start time is in the future, return an error indicating the form isn't open yet.
                if saved_start_time.gt(&end_time) {
                    return Err(ContractError::CustomError {
                        msg: format!("Not opened yet. Will be opened at {:?}", saved_start_time),
                    });
                }
                // Otherwise, set the end time to the current end_time value to close the form.
                else {
                    config.end_time = Some(end_time);
                    CONFIG.save(ctx.deps.storage, &config)?;
                }
            }
            // If no start time exists:
            None => {
                // Return an error indicating the form has not been opened yet.
                return Err(ContractError::CustomError {
                    msg: "Not opened yet".to_string(),
                });
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

pub fn validate_form_is_opened(env: Env, config: Config) -> Result<(), ContractError> {
    let current_time = Milliseconds::from_nanos(env.block.time.nanos());
    let saved_start_time = config.start_time;
    let saved_end_time = config.end_time;
    match saved_start_time {
        Some(saved_start_time) => {
            if saved_start_time.gt(&current_time) {
                return Err(ContractError::CustomError {
                    msg: format!("Not opened yet. Will be opened at {:?}", saved_start_time),
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
