use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Binary, Coin, ContractResult, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};

use crate::primitive_keys::{ANCHOR_AUST, ANCHOR_MARKET};
use andromeda_protocol::primitive::QueryMsg as PrimitiveQueryMsg;
use common::{
    ado_base::AndromedaQuery,
    primitive::{GetValueResponse, Primitive},
};
use cw20::{BalanceResponse, Cw20QueryMsg};
use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_MARKET_CONTRACT: &str = "anchor_market";
pub const MOCK_AUST_TOKEN: &str = "aust_token";

pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";

pub struct WasmMockQuerier {
    pub base: MockQuerier<TerraQueryWrapper>,
    pub token_balance: Uint128,
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
                    MOCK_AUST_TOKEN => self.handle_aust_query(msg),
                    MOCK_PRIMITIVE_CONTRACT => self.handle_primitive_query(msg),
                    _ => panic!("Unsupported Query for address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_aust_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw20QueryMsg::Balance { .. } => {
                let res = BalanceResponse {
                    balance: self.token_balance,
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_primitive_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            PrimitiveQueryMsg::AndrQuery(AndromedaQuery::Get(data)) => {
                let key: String = from_binary(&data.unwrap()).unwrap();
                let msg_response = match key.as_str() {
                    ANCHOR_MARKET => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_MARKET_CONTRACT.to_owned()),
                    },
                    ANCHOR_AUST => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_AUST_TOKEN.to_owned()),
                    },
                    _ => panic!("Unsupported primitive key"),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            token_balance: Uint128::zero(),
        }
    }
}
