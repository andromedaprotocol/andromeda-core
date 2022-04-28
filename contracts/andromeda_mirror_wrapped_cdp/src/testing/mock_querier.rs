use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};

use crate::primitive_keys::{MIRROR_GOV, MIRROR_LOCK, MIRROR_MINT, MIRROR_MIR, MIRROR_STAKING};
use andromeda_protocol::primitive::QueryMsg as PrimitiveQueryMsg;
use common::{
    ado_base::AndromedaQuery,
    primitive::{GetValueResponse, Primitive},
};
pub use mirror_protocol::{
    collateral_oracle::{
        ConfigResponse as CollateralOracleConfigResponse,
        QueryMsg as MirrorCollateralOracleQueryMsg,
    },
    gov::{ConfigResponse as GovConfigResponse, QueryMsg as MirrorGovQueryMsg},
    lock::{ConfigResponse as LockConfigResponse, QueryMsg as MirrorLockQueryMsg},
    mint::{ConfigResponse as MintConfigResponse, QueryMsg as MirrorMintQueryMsg},
    oracle::{ConfigResponse as OracleConfigResponse, QueryMsg as MirrorOracleQueryMsg},
    staking::{ConfigResponse as StakingConfigResponse, QueryMsg as MirrorStakingQueryMsg},
};
use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
pub const MOCK_MIRROR_TOKEN_ADDR: &str = "mirror_token";
pub const MOCK_MIRROR_MINT_ADDR: &str = "mirror_mint";
pub const MOCK_MIRROR_STAKING_ADDR: &str = "mirror_staking";
pub const MOCK_MIRROR_GOV_ADDR: &str = "mirror_gov";
pub const MOCK_MIRROR_LOCK_ADDR: &str = "mirror_lock";

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
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_PRIMITIVE_CONTRACT => self.handle_primitive_query(msg),
                    _ => panic!("Unsupported Query for address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_primitive_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            PrimitiveQueryMsg::AndrQuery(AndromedaQuery::Get(data)) => {
                let key: String = from_binary(&data.unwrap()).unwrap();
                let msg_response = match key.as_str() {
                    MIRROR_MINT => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_MIRROR_MINT_ADDR.to_owned()),
                    },
                    MIRROR_STAKING => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_MIRROR_STAKING_ADDR.to_owned()),
                    },
                    MIRROR_GOV => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_MIRROR_GOV_ADDR.to_owned()),
                    },
                    MIRROR_LOCK => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_MIRROR_LOCK_ADDR.to_owned()),
                    },
                    MIRROR_MIR => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_MIRROR_TOKEN_ADDR.to_owned()),
                    },
                    _ => panic!("Unsupported primitive key"),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier { base }
    }
}
