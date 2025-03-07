use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
use andromeda_std::{
    ado_base::InstantiateMsg, ado_contract::ADOContract,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::QuerierWrapper;
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    Coin, ContractInfoResponse, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError,
    SystemResult, WasmQuery,
};
use cosmwasm_std::{to_json_binary, ContractResult};

pub const MOCK_CW721_CONTRACT: &str = "cw721";

/// Alternative to `cosmwasm_std::testing::mock_dependencies` that allows us to respond to custom queries.
///
/// Automatically assigns a kernel address as MOCK_KERNEL_CONTRACT.
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
            &QuerierWrapper::new(&deps.querier),
            mock_info("sender", &[]),
            InstantiateMsg {
                ado_type: "vdf-mint".to_string(),
                ado_version: "test".to_string(),
                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();
    deps
}
pub struct WasmMockQuerier {
    pub base: MockQuerier,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<cosmwasm_std::Empty> = match from_json(bin_request) {
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
    pub fn handle_query(&self, request: &QueryRequest<cosmwasm_std::Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, .. }) => {
                match contract_addr.as_str() {
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            QueryRequest::Wasm(WasmQuery::ContractInfo { contract_addr }) => {
                match contract_addr.as_str() {
                    MOCK_CW721_CONTRACT => self.handle_cw721_contract_info_query(),
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
        }
    }

    fn handle_cw721_contract_info_query(&self) -> QuerierResult {
        let mut msg_response = ContractInfoResponse::default();
        msg_response.code_id = 4;
        SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
    }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}
