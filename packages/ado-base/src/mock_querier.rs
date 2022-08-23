use common::{
    ado_base::{AndromedaQuery, QueryMsg},
    primitive::{GetValueResponse, Primitive},
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};
use cw20::{BalanceResponse, Cw20QueryMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MOCK_CW20_CONTRACT: &str = "cw20_contract";
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
pub const MOCK_APP_CONTRACT: &str = "app_contract";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum MissionQuery {
    AndrQuery(AndromedaQuery),
    ComponentExists { name: String },
}

pub struct WasmMockQuerier {
    pub base: MockQuerier,
}

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
                    MOCK_CW20_CONTRACT => self.handle_cw20_query(msg),
                    MOCK_PRIMITIVE_CONTRACT => self.handle_primitive_query(msg),
                    MOCK_APP_CONTRACT => self.handle_app_query(msg),
                    _ => panic!("Unsupported query for contract: {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_cw20_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw20QueryMsg::Balance { .. } => {
                let balance_response = BalanceResponse {
                    balance: 10u128.into(),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&balance_response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_primitive_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            QueryMsg::AndrQuery(AndromedaQuery::Get(data)) => {
                let key: String = from_binary(&data.unwrap()).unwrap();
                let msg_response = match key.as_str() {
                    "key1" => GetValueResponse {
                        key,
                        value: Primitive::String("address1".to_string()),
                    },
                    "key2" => GetValueResponse {
                        key,
                        value: Primitive::String("address2".to_string()),
                    },
                    _ => panic!("Unsupported primitive key"),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_app_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MissionQuery::AndrQuery(AndromedaQuery::Get(_)) => {
                SystemResult::Ok(ContractResult::Ok(to_binary(&"actual_address").unwrap()))
            }
            MissionQuery::ComponentExists { name } => {
                let value = name == "a";
                SystemResult::Ok(ContractResult::Ok(to_binary(&value).unwrap()))
            }
            _ => SystemResult::Ok(ContractResult::Err("Error".to_string())),
        }
    }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}
