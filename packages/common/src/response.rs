use crate::error::ContractError;
use cosmwasm_std::Reply;
use cw_utils::parse_reply_instantiate_data;

pub fn get_reply_address(msg: Reply) -> Result<String, ContractError> {
    let res = parse_reply_instantiate_data(msg)?;
    Ok(res.contract_address)
}
