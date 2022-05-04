use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Binary, Coin, ContractResult, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};

use crate::primitive_keys::{
    ANCHOR_ANC, ANCHOR_BLUNA, ANCHOR_BLUNA_CUSTODY, ANCHOR_BLUNA_HUB, ANCHOR_GOV, ANCHOR_MARKET,
    ANCHOR_ORACLE, ANCHOR_OVERSEER,
};
use anchor_token::gov::{QueryMsg as GovQueryMsg, StakerResponse};
use andromeda_protocol::{
    anchor::{BLunaHubQueryMsg, WithdrawableUnbondedResponse},
    primitive::QueryMsg as PrimitiveQueryMsg,
};
use common::{
    ado_base::AndromedaQuery,
    primitive::{GetValueResponse, Primitive},
};
use cw20::{BalanceResponse, Cw20QueryMsg};
use moneymarket::{
    market::{BorrowerInfoResponse, QueryMsg as MarketQueryMsg},
    oracle::{PriceResponse, QueryMsg as OracleQueryMsg},
    overseer::{CollateralsResponse, QueryMsg as OverseerQueryMsg},
};
use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_MARKET_CONTRACT: &str = "anchor_market";
pub const MOCK_CUSTODY_CONTRACT: &str = "anchor_custody";
pub const MOCK_OVERSEER_CONTRACT: &str = "anchor_overseer";
pub const MOCK_BLUNA_TOKEN: &str = "bluna_token";
pub const MOCK_ANC_TOKEN: &str = "anc_token";
pub const MOCK_GOV_CONTRACT: &str = "anchor_gov";
pub const MOCK_BLUNA_HUB_CONTRACT: &str = "bluna_hub_contract";
pub const MOCK_ORACLE_CONTRACT: &str = "anchor_oracle";

pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";

pub struct WasmMockQuerier {
    pub base: MockQuerier<TerraQueryWrapper>,
    pub loan_amount: Uint256,
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
                    MOCK_MARKET_CONTRACT => self.handle_market_query(msg),
                    MOCK_OVERSEER_CONTRACT => self.handle_overseer_query(msg),
                    MOCK_ORACLE_CONTRACT => self.handle_oracle_query(msg),
                    MOCK_BLUNA_HUB_CONTRACT => self.handle_bluna_hub_query(msg),
                    MOCK_GOV_CONTRACT => self.handle_gov_query(msg),
                    MOCK_BLUNA_TOKEN => self.handle_bluna_query(msg),
                    MOCK_ANC_TOKEN => self.handle_anc_query(msg),
                    MOCK_PRIMITIVE_CONTRACT => self.handle_primitive_query(msg),
                    _ => panic!("Unsupported Query for address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_bluna_hub_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            BLunaHubQueryMsg::WithdrawableUnbonded { .. } => {
                let res = WithdrawableUnbondedResponse {
                    withdrawable: Uint128::new(100),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
        }
    }

    fn handle_market_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MarketQueryMsg::BorrowerInfo { borrower, .. } => {
                let res = BorrowerInfoResponse {
                    borrower,
                    interest_index: Decimal256::zero(),
                    reward_index: Decimal256::zero(),
                    loan_amount: self.loan_amount,
                    pending_rewards: Decimal256::from_uint256(Uint256::from(200u128)),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_overseer_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            OverseerQueryMsg::Collaterals { borrower } => {
                let res = CollateralsResponse {
                    borrower,
                    collaterals: vec![(MOCK_BLUNA_TOKEN.to_owned(), Uint256::from(100u128))],
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_oracle_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            OracleQueryMsg::Price { .. } => {
                let res = PriceResponse {
                    rate: Decimal256::one(),
                    last_updated_base: 0,
                    last_updated_quote: 0,
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_bluna_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw20QueryMsg::Balance { .. } => {
                let res = BalanceResponse {
                    balance: 100u128.into(),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_anc_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw20QueryMsg::Balance { .. } => {
                let res = BalanceResponse {
                    balance: 100u128.into(),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_gov_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            GovQueryMsg::Staker { .. } => {
                let res = StakerResponse {
                    balance: Uint128::new(100),
                    share: Uint128::zero(),
                    locked_balance: vec![],
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
                    ANCHOR_OVERSEER => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_OVERSEER_CONTRACT.to_owned()),
                    },
                    ANCHOR_BLUNA_HUB => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_BLUNA_HUB_CONTRACT.to_owned()),
                    },
                    ANCHOR_BLUNA_CUSTODY => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_CUSTODY_CONTRACT.to_owned()),
                    },
                    ANCHOR_ORACLE => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_ORACLE_CONTRACT.to_owned()),
                    },
                    ANCHOR_GOV => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_GOV_CONTRACT.to_owned()),
                    },
                    ANCHOR_BLUNA => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_BLUNA_TOKEN.to_owned()),
                    },
                    ANCHOR_ANC => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_ANC_TOKEN.to_owned()),
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
            loan_amount: Uint256::zero(),
        }
    }
}
