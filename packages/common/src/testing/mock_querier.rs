use crate::{
    ado_base::{AndromedaQuery, QueryMsg},
    primitive::{GetValueResponse, Primitive},
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, Decimal, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_MISSION_CONTRACT: &str = "mission_contract";
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";

pub struct WasmMockQuerier {
    pub base: MockQuerier<TerraQueryWrapper>,
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
    }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
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
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_MISSION_CONTRACT => self.handle_mission_query(msg),
                    MOCK_PRIMITIVE_CONTRACT => self.handle_primitive_query(msg),
                    _ => panic!("Unsupported query for contract: {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_mission_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            QueryMsg::AndrQuery(AndromedaQuery::Get(_)) => {
                SystemResult::Ok(ContractResult::Ok(to_binary(&"actual_address").unwrap()))
            }
            _ => SystemResult::Ok(ContractResult::Err("Error".to_string())),
        }
    }

    fn handle_primitive_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            QueryMsg::AndrQuery(AndromedaQuery::Get(data)) => {
                let res = match data {
                    None => GetValueResponse {
                        key: "default".to_string(),
                        value: Primitive::Decimal(Decimal::zero()),
                    },
                    Some(data) => {
                        let key: String = from_binary(&data).unwrap();
                        match key.as_str() {
                            "String" => GetValueResponse {
                                key,
                                value: Primitive::String("Value".to_string()),
                            },
                            "Uint128" => GetValueResponse {
                                key,
                                value: Primitive::Uint128(Uint128::new(10)),
                            },
                            "Decimal" => GetValueResponse {
                                key,
                                value: Primitive::Decimal(Decimal::percent(1)),
                            },
                            "Coin" => GetValueResponse {
                                key,
                                value: Primitive::Coin(Coin::new(100, "uusd")),
                            },
                            "Bool" => GetValueResponse {
                                key,
                                value: Primitive::Bool(true),
                            },
                            "Vec" => GetValueResponse {
                                key,
                                value: Primitive::Vec(vec![Primitive::from("String".to_string())]),
                            },
                            _ => {
                                return SystemResult::Ok(ContractResult::Err(
                                    "Not Found".to_string(),
                                ))
                            }
                        }
                    }
                };

                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier { base }
    }
}
