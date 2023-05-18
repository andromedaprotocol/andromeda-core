use andromeda_fungible_tokens::cw20::QueryMsg as Cw20Query;

use andromeda_std::ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse};
use andromeda_std::ado_base::InstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::Funds;
use andromeda_std::testing::mock_querier::WasmMockQuerier as AndrMockQuerier;
use andromeda_std::{
    ado_base::modules::Module,
    amp::addresses::AndrAddr,
    common::{encode_binary, expiration::MILLISECONDS_TO_NANOSECONDS_RATIO},
    error::ContractError,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::testing::{
    mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Empty, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};
use std::collections::HashMap;

pub fn mock_dependencies_custom(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));
    let storage = MockStorage::default();
    let mut deps = OwnedDeps {
        storage,
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: std::marker::PhantomData,
    };
    ADOContract::default()
        .instantiate(
            &mut deps.storage,
            mock_env(),
            &deps.api,
            mock_info("sender", &[]),
            InstantiateMsg {
                ado_type: "cw20-staking".to_string(),
                ado_version: "test".to_string(),
                operators: None,
                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();
    deps
}

pub struct WasmMockQuerier {
    pub base: MockQuerier<Empty>,
    token_querier: TokenQuerier,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {e}"),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(msg).unwrap() {
                    // Cw20QueryMsg::Balance { address } => {
                    //     let balances: &HashMap<String, Uint128> =
                    //         match self.token_querier.balances.get(contract_addr) {
                    //             Some(balances) => balances,
                    //             None => {
                    //                 return SystemResult::Err(SystemError::InvalidRequest {
                    //                     error: format!(
                    //                     "No balance info exists for the contract {contract_addr}"
                    //                 ),
                    //                     request: msg.as_slice().into(),
                    //                 })
                    //             }
                    //         };

                    //     let balance = match balances.get(&address) {
                    //         Some(v) => *v,
                    //         None => {
                    //             return SystemResult::Ok(ContractResult::Ok(
                    //                 to_binary(&Cw20BalanceResponse {
                    //                     balance: Uint128::zero(),
                    //                 })
                    //                 .unwrap(),
                    //             ));
                    //         }
                    //     };

                    //     SystemResult::Ok(ContractResult::Ok(
                    //         to_binary(&Cw20BalanceResponse { balance }).unwrap(),
                    //     ))
                    // }
                    Cw20Query::Owner {} => SystemResult::Ok(ContractResult::Ok(
                        to_binary(&andromeda_std::ado_base::ownership::ContractOwnerResponse {
                            owner: "owner".to_string(),
                        })
                        .unwrap(),
                    )),
                    _ => AndrMockQuerier::new(MockQuerier::new(&[])).handle_query(request),
                }
            }
            _ => AndrMockQuerier::new(MockQuerier::new(&[])).handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }
}
