use crate::error::ContractError;
use cosmwasm_std::{to_json_binary, Reply};
use cw_utils::parse_instantiate_response_data;

pub fn get_reply_address(msg: Reply) -> Result<String, ContractError> {
    //TODO confirm if this is correct
    let res = parse_instantiate_response_data(&to_json_binary(&msg)?)?;
    Ok(res.contract_address)
}
