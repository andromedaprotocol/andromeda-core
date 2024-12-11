use andromeda_data_storage::form::GetSchemaResponse;
use andromeda_modules::schema::{QueryMsg as SchemaQueryMsg, ValidateDataResponse};
use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
use andromeda_std::{
    ado_base::InstantiateMsg, ado_contract::ADOContract,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::QuerierWrapper;
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    Coin, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};
use cosmwasm_std::{to_json_binary, Binary, ContractResult};

pub const MOCK_SCHEMA_ADO: &str = "schema_ado";

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
                ado_type: "form".to_string(),
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
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_SCHEMA_ADO => self.handle_schema_smart_query(msg),
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
        }
    }

    fn handle_schema_smart_query(&self, msg: &Binary) -> QuerierResult {
        match from_json(msg).unwrap() {
            SchemaQueryMsg::GetSchema {} => {
                let msg_response = GetSchemaResponse {
                    schema: "{\"properties\":{\"age\":{\"type\":\"number\"},\"name\":{\"type\":\"string\"}},\"required\":[\"name\",\"age\"],\"type\":\"object\"}".to_string(),
                };
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
            }
            SchemaQueryMsg::ValidateData { data } => match data.as_str().starts_with("valid") {
                true => {
                    let msg_response = ValidateDataResponse::Valid;
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
                }
                false => {
                    let msg_response = ValidateDataResponse::Invalid {
                        msg: "Invalid data against schema".to_string(),
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
                }
            },
            _ => panic!("Unsupported Query"),
        }
    }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}
