use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, ContractInfoResponse, ContractResult, OwnedDeps,
    Querier, QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};

pub fn mock_dependencies_custom_v2(
    _contract_balance: &[cosmwasm_std::Coin],
) -> OwnedDeps<MockStorage, MockApi, CustomWasmMockQuerier> {
    let custom_querier = CustomWasmMockQuerier::new();
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: std::marker::PhantomData,
    }
}

pub struct CustomWasmMockQuerier {
    base: MockQuerier,
}

impl CustomWasmMockQuerier {
    pub fn new() -> Self {
        CustomWasmMockQuerier {
            base: MockQuerier::new(&[]),
        }
    }

    fn handle_wasm_query(&self, query: &WasmQuery) -> QuerierResult {
        match query {
            WasmQuery::Smart {
                contract_addr,
                msg: _,
            } => {
                if contract_addr == MOCK_KERNEL_CONTRACT {
                    // Handle kernel queries
                    let response = ContractInfoResponse::new(
                        1,                              // code_id
                        Addr::unchecked("creator"),     // creator
                        Some(Addr::unchecked("admin")), // admin
                        false,                          // pinned
                        Some("ibc-port".to_string()),   // ibc_port
                    );
                    let res = to_json_binary(&response).unwrap();
                    SystemResult::Ok(ContractResult::Ok(res))
                } else {
                    // Mock response for other contracts
                    let response = ContractInfoResponse::new(
                        1,                              // code_id
                        Addr::unchecked("creator"),     // creator
                        Some(Addr::unchecked("admin")), // admin
                        false,                          // pinned
                        None,                           // ibc_port
                    );
                    let res = to_json_binary(&response).unwrap();
                    SystemResult::Ok(ContractResult::Ok(res))
                }
            }
            WasmQuery::Raw { .. } => {
                // Return a default valid response for any raw query
                SystemResult::Ok(ContractResult::Ok(Binary::default()))
            }
            WasmQuery::ContractInfo { .. } => {
                // Mock contract info response
                let response = ContractInfoResponse::new(
                    1,                              // code_id
                    Addr::unchecked("creator"),     // creator
                    Some(Addr::unchecked("admin")), // admin
                    false,                          // pinned
                    None,                           // ibc_port
                );
                let res = to_json_binary(&response).unwrap();
                SystemResult::Ok(ContractResult::Ok(res))
            }
            _ => SystemResult::Err(SystemError::InvalidRequest {
                error: "Unsupported WasmQuery".to_string(),
                request: Default::default(),
            }),
        }
    }
}

impl Querier for CustomWasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<cosmwasm_std::Empty> = match from_json(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };

        match request {
            QueryRequest::Wasm(wasm_query) => self.handle_wasm_query(&wasm_query),
            _ => self.base.raw_query(bin_request),
        }
    }
}
