use andromeda_automation::evaluation::QueryMsg as EvaluationQueryMsg;
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};

pub const LEGIT_ADDRESS1: &str = "legit_address1";
pub const LEGIT_ADDRESS2: &str = "legit_address2";

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
                    LEGIT_ADDRESS1 => self.handle_evaluation_query(msg),
                    LEGIT_ADDRESS2 => self.handle_evaluation2_query(msg),
                    _ => panic!("Unknown Contract Address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_evaluation_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            EvaluationQueryMsg::Evaluation {} => {
                let res = true;
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }

            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_evaluation2_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            EvaluationQueryMsg::Evaluation {} => {
                let res = false;
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }

            _ => panic!("Unsupported Query"),
        }
    }

    pub fn new(base: MockQuerier<cosmwasm_std::Empty>) -> Self {
        WasmMockQuerier { base }
    }
}
