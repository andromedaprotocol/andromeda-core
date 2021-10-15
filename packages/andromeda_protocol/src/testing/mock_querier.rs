use crate::{address_list::IncludesAddressResponse, ownership::ContractOwnerResponse};
use cosmwasm_std::{
    from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};
use terra_cosmwasm::TerraQueryWrapper;

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

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
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
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg: _,
            }) => {
                if contract_addr == &Addr::unchecked("addresslist_contract_address1") {
                    let msg_response = IncludesAddressResponse { included: true };
                    return SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()));
                } else if contract_addr == &Addr::unchecked("factory_address") {
                    let msg_response = ContractOwnerResponse {
                        owner: String::from("creator"),
                    };
                    return SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()));
                } else {
                    let msg_response = IncludesAddressResponse { included: false };
                    return SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()));
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier { base }
    }
}
