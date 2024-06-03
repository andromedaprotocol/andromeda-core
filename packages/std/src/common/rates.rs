use cosmwasm_std::{from_json, BankMsg, CosmosMsg, SubMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;

/// Gets the amount of tax paid by iterating over the `msgs` and comparing it to the
/// difference between the base amount and the amount left over after royalties.
/// It is assumed that each bank message has a single Coin to send as transfer
/// agreements only accept a single Coin. It is also assumed that the result will always be
/// non-negative.
///
/// # Arguments
///
/// * `msgs` - The vector of submessages containing fund transfers
/// * `base_amount` - The amount paid before tax.
/// * `remaining_amount_after_royalties` - The amount remaining of the base_amount after royalties
///                                        are applied
/// Returns the amount of tax necessary to be paid on top of the `base_amount`.
pub fn get_tax_amount(
    msgs: &[SubMsg],
    base_amount: Uint128,
    remaining_amount_after_royalties: Uint128,
) -> Uint128 {
    let deducted_amount = base_amount - remaining_amount_after_royalties;
    msgs.iter()
        .map(|msg| {
            if let CosmosMsg::Bank(BankMsg::Send { amount, .. }) = &msg.msg {
                amount[0].amount
            } else {
                Uint128::zero()
            }
        })
        .reduce(|total, amount| total + amount)
        .unwrap_or_else(Uint128::zero)
        - deducted_amount
}

pub fn get_tax_amount_cw20(
    msgs: &[SubMsg],
    base_amount: Uint128,
    remaining_amount_after_royalties: Uint128,
) -> Uint128 {
    let deducted_amount = base_amount - remaining_amount_after_royalties;
    msgs.iter()
        .map(|msg| {
            if let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &msg.msg {
                if let Ok(Cw20ExecuteMsg::Transfer { amount, .. }) = from_json(msg) {
                    return amount;
                }
            }
            Uint128::zero()
        })
        .reduce(|total, amount| total + amount)
        .unwrap_or_else(Uint128::zero)
        - deducted_amount
}
