use cosmwasm_std::{Reply, StdError, StdResult};

pub fn get_reply_address(msg: &Reply) -> StdResult<String> {
    let res = msg.result.clone().unwrap();
    let events = &res.events;
    for event in events.iter() {
        for attr in event.attributes.iter() {
            if attr.key == "contract_address" {
                return Ok(attr.value.clone());
            }
        }
    }
    Err(StdError::generic_err(
        "Could not parse reply contract address",
    ))
}
