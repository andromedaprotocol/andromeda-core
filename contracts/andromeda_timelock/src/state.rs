use cw_storage_plus::{ Item, Map, };
use schemars::JsonSchema;
use serde::{ Deserialize, Serialize, };
use cosmwasm_std::{ Addr, Coin, Storage,StdResult };
use cw721::{ Expiration };

pub const STATE: Item<State> = Item::new("state");

pub const HOLDED_FUNDS: Map<String, HoldFunds> = Map::new("funds");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HoldFunds {
    pub coins: Vec<Coin>,
    pub expire: Expiration,
}

impl HoldFunds {
    pub fn hold_funds(&self,storage: &mut dyn Storage, addr: String)->StdResult<()>{
        HOLDED_FUNDS.save( storage,addr.clone(), &self )
    }
    pub fn relase_hold_funds(storage: &mut dyn Storage, addr: String){
        HOLDED_FUNDS.remove( storage, addr.clone() );
    }
    pub fn get_funds(storage: &dyn Storage, addr: String) -> StdResult<Option<HoldFunds>> {
        HOLDED_FUNDS.may_load( storage, addr )
    }
}
