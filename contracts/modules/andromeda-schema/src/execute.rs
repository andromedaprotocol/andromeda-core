use andromeda_modules::schema::ExecuteMsg;
use andromeda_std::{
    ado_contract::ADOContract,
    common::{actions::call_action, context::ExecuteContext},
    error::ContractError,
};
use cosmwasm_std::{ensure, Response};
use cw_json::JSON;
use serde_json::{from_str, Value};

use crate::state::SCHEMA;

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;

    let res = match msg {
        ExecuteMsg::UpdateSchema {
            new_schema_json_string,
        } => execute_update_schema(ctx, new_schema_json_string),
        _ => ADOContract::default().execute(ctx, msg),
    }?;

    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

fn execute_update_schema(
    ctx: ExecuteContext,
    new_schema_json: String,
) -> Result<Response, ContractError> {
    let sender: cosmwasm_std::Addr = ctx.info.sender;

    ensure!(
        ADOContract::default().is_owner_or_operator(ctx.deps.storage, sender.as_ref())?,
        ContractError::Unauthorized {}
    );

    let new_schema_json_value: Value =
        from_str(new_schema_json.as_str()).map_err(|_| ContractError::CustomError {
            msg: "Invalid JSON Schema".to_string(),
        })?;
    let new_schema_json = JSON::try_from(new_schema_json_value.to_string().as_str()).unwrap();

    SCHEMA.save(ctx.deps.storage, &new_schema_json)?;

    let response = Response::new()
        .add_attribute("method", "update_schema")
        .add_attribute("sender", sender);

    Ok(response)
}
