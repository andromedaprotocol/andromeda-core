use crate::error::ContractError;
use cosmwasm_std::Reply;
use cw_utils::parse_instantiate_response_data;

pub fn get_reply_address(msg: Reply) -> Result<String, ContractError> {
    let msg_clone = msg.clone();
    let result = msg_clone.result.unwrap();
    #[allow(deprecated)]
    let res = parse_instantiate_response_data(result.data.unwrap().as_slice()).or_else(|_| {
        let data = result.msg_responses[0].value.as_slice();
        parse_instantiate_response_data(data)
    })?;
    Ok(res.contract_address)
}
