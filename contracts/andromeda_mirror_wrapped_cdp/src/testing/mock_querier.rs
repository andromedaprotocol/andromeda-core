use andromeda_protocol::mirror_wrapped_cdp::MirrorMintQueryMsg;
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, Coin, ContractResult, Decimal, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, WasmQuery,
};
use mirror_protocol::mint::ConfigResponse;
use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_MIRROR_MINT_ADDR: &str = "mirror_mint";
pub const MOCK_MIRROR_STAKING_ADDR: &str = "mirror_staking";
pub const MOCK_MIRROR_GOV_ADDR: &str = "mirror_gov";

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
        let mock_config_response = ConfigResponse {
            owner: "owner".to_string(),
            oracle: "oracle".to_string(),
            collector: "collector".to_string(),
            collateral_oracle: "collateral_oracle".to_string(),
            staking: "staking".to_string(),
            terraswap_factory: "terraswap_factory".to_string(),
            lock: "lock".to_string(),
            base_denom: "base_denom".to_string(),
            token_code_id: 1_u64,
            protocol_fee_rate: Decimal::one(),
        };
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                if contract_addr == MOCK_MIRROR_MINT_ADDR {
                    match from_binary(msg).unwrap() {
                        MirrorMintQueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                            to_binary(&mock_config_response).unwrap(),
                        )),
                        _ => self.base.handle_query(request),
                    }
                } else {
                    panic!();
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier { base }
    }
}
