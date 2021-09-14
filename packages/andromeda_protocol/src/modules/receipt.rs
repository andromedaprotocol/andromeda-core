use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult, Storage, BankMsg, to_binary, Uint128};
use cw_storage_plus::{Item, Map, U128Key};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::token::{ TokenId , ExecuteMsg };
use crate::modules::{ModuleDefinition, Module, };
use crate::modules::hooks::{MessageHooks, HookResponse, };
use crate::hook::InitHook;

pub const RECEIPT: Map<U128Key, ReceiptData> = Map::new("receipt");
pub const NUM_RECEIPTS: Item<u128> = Item::new("numreceipts");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Receipt;

impl Module for Receipt {
    fn validate(&self, _extensions: Vec<ModuleDefinition>) -> StdResult<bool>{
        Ok(true)
    }
    fn as_definition(&self) -> ModuleDefinition {
        ModuleDefinition::Receipt
    }
}

impl MessageHooks for Receipt {
    fn on_store_receipt(
        &self,
        env: Env,
        token_id: TokenId,
        owner: String,
        purchaser: String,
        payments: &Vec<BankMsg>,
    ) -> StdResult<HookResponse>{
        let mut hook_response = HookResponse::default();
        
        let mut transfer_data:Vec<TransferData> = vec![];
        for payment in payments {
            match payment {
                BankMsg::Send{ to_address, amount} =>{
                    transfer_data.push(TransferData{
                        amount: amount[0].amount,
                        denom: amount[0].denom.clone(),
                        receiver: to_address.clone()
                    });
                },
                _ => {}
            }            
        }

        let msg = to_binary(&ExecuteMsg::Receipt {
            token_id,
            seller: owner,
            purchaser,
            transfer_data
        })?;

        hook_response.add_message(&InitHook{
            msg,
            contract_addr: env.contract.address.to_string(),
        }.into_cosmos_msg()?);
        Ok(hook_response)
    }
}



#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ReceiptData {
    pub token_id: TokenId,
    pub seller: String,
    pub purchaser: String,
    pub transfer_data: Vec<TransferData>        
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransferData {
    pub amount: Uint128,
    pub denom: String,
    pub receiver: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReceiptResponse {
    pub receipt: ReceiptData,
}


pub fn execute_receipt(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    token_id: TokenId,    
    seller: String, //address
    purchaser: String, // address
    transfer_data: Vec<TransferData>
) -> StdResult<Response> {

    let receipt = ReceiptData{
        token_id,        
        seller,
        purchaser,
        transfer_data
    };
    let receipt_id:u128 = get_num_receipts(deps.storage);
    RECEIPT.save(deps.storage, U128Key::from(receipt_id), &receipt)?;
    increment_num_receipts(deps.storage)?;
    Ok(Response::default())
}

pub fn increment_num_receipts(storage: &mut dyn Storage) -> StdResult<()> {
    let receipt_count = NUM_RECEIPTS.load(storage).unwrap_or_default();
    NUM_RECEIPTS.save(storage, &(receipt_count + 1))
}
pub fn get_num_receipts(storage: &dyn Storage) -> u128 {
    NUM_RECEIPTS.load(storage).unwrap_or_default()    
}

pub fn query_receipt(storage: &dyn Storage, receipt_id: u128) -> StdResult<ReceiptResponse>{
    let receipt: ReceiptData = RECEIPT.load(storage, U128Key::from(receipt_id))?;
    
    Ok(ReceiptResponse { receipt })
}


#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins,coin,
        testing::{mock_env},
        BankMsg,        
    };

    use super::*;

    #[test]
    fn test_receipt_validate() {
        let t = Receipt{};
        assert_eq!(t.validate(vec![]).unwrap(), true);       
    }

    #[test]

    fn test_receipt_on_store_receipt() {
        
        let env = mock_env();
        let r = Receipt {};       
        let token_id:String = "token1".to_string();
        let agreed_transfer_amount = coin(100, "uluna");
        let tax_amount = 1;
        let owner = String::from("owner");
        let purchaser = String::from("purchaser");
        let payments = vec![
            BankMsg::Send {
                to_address: String::from("recv1"),
                amount: coins(tax_amount, &agreed_transfer_amount.denom.to_string()),
            },
            BankMsg::Send {
                to_address: String::from("recv2"),
                amount: coins(tax_amount, &agreed_transfer_amount.denom.to_string()),
            }, 
        ];
        
        let hook_response =  r.on_store_receipt(            
            env.clone(),
            token_id.clone(),
            owner.clone(),
            purchaser.clone(),            
            &payments,
        ).unwrap();

        assert_ne!(hook_response, HookResponse::default());        
    }
}
