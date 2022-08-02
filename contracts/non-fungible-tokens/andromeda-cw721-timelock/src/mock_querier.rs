use cosmwasm_std::{
    from_binary, from_slice,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};
use cw721::{Cw721QueryMsg, OwnerOfResponse};

pub const MOCK_TOKEN_ADDR: &str = "token0001";
pub const MOCK_TOKEN_OWNER: &str = "owner";
pub const MOCK_UNCLAIMED_TOKEN: &str = "unclaimed_token";

pub fn mock_dependencies_custom(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: std::marker::PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<cosmwasm_std::Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<cosmwasm_std::Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_TOKEN_ADDR => self.handle_token_query(msg),
                    _ => panic!("Unknown Contract Address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_token_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw721QueryMsg::OwnerOf { token_id, .. } => {
                let res = if token_id == MOCK_UNCLAIMED_TOKEN {
                    OwnerOfResponse {
                        owner: mock_env().contract.address.to_string(),
                        approvals: vec![],
                    }
                } else {
                    OwnerOfResponse {
                        owner: MOCK_TOKEN_OWNER.to_owned(),
                        approvals: vec![],
                    }
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }

            _ => panic!("Unsupported Query"),
        }
    }

    pub fn new(base: MockQuerier<cosmwasm_std::Empty>) -> Self {
        WasmMockQuerier { base }
    }
}
