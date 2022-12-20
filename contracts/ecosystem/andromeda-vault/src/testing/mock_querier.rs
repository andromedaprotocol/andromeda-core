//use andromeda_ecosystem::anchor_earn::PositionResponse;
use common::ado_base::{
    operators::IsOperatorResponse, recipient::Recipient, AndromedaQuery, QueryMsg,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, Uint128, WasmQuery,
};

// This is here since anchor_earn is defunct now.
#[cw_serde]
pub struct PositionResponse {
    pub recipient: Recipient,
    pub aust_amount: Uint128,
}

pub const MOCK_ANCHOR_CONTRACT: &str = "anchor_contract";
pub const MOCK_VAULT_CONTRACT: &str = "vault_contract";

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
    pub base: MockQuerier,
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
                    MOCK_ANCHOR_CONTRACT => self.handle_anchor_balance_query(msg),
                    _ => panic!("DO NOT ENTER HERE"),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_anchor_balance_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            QueryMsg::AndrQuery(andr_msg) => match andr_msg {
                AndromedaQuery::Get(data) => {
                    let recipient: String = from_binary(&data.unwrap()).unwrap();
                    let msg_response = PositionResponse {
                        recipient: Recipient::Addr(recipient),
                        aust_amount: Uint128::from(10u128),
                    };
                    SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
                }
                AndromedaQuery::IsOperator { address } => {
                    let msg_response = IsOperatorResponse {
                        is_operator: address == MOCK_VAULT_CONTRACT,
                    };
                    SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
                }
                _ => panic!("Unsupported Query"),
            },
        }
    }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}
