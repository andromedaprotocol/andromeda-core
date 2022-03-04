use andromeda_protocol::anchor::{BLunaHubQueryMsg, BLunaHubStateResponse};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Binary, Coin, ContractResult, Decimal, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse, Cw20QueryMsg};
use moneymarket::{
    custody::{BAssetInfo, ConfigResponse as CustodyConfigResponse, QueryMsg as CustodyQueryMsg},
    market::{
        BorrowerInfoResponse, ConfigResponse as MarketConfigResponse, QueryMsg as MarketQueryMsg,
    },
    oracle::{PriceResponse, QueryMsg as OracleQueryMsg},
    overseer::{
        CollateralsResponse, ConfigResponse as OverseerConfigResponse, QueryMsg as OverseerQueryMsg,
    },
};
use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_MARKET_CONTRACT: &str = "anchor_market";
pub const MOCK_CUSTODY_CONTRACT: &str = "anchor_custody";
pub const MOCK_OVERSEER_CONTRACT: &str = "anchor_overseer";
pub const MOCK_AUST_TOKEN: &str = "aust_token";
pub const MOCK_BLUNA_TOKEN: &str = "bluna_token";
pub const MOCK_BLUNA_HUB_CONTRACT: &str = "bluna_hub_contract";
pub const MOCK_ORACLE_CONTRACT: &str = "anchor_oracle";

pub struct WasmMockQuerier {
    pub base: MockQuerier<TerraQueryWrapper>,
    pub token_balance: Uint128,
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
                    MOCK_MARKET_CONTRACT => self.handle_market_query(&msg),
                    MOCK_CUSTODY_CONTRACT => self.handle_custody_query(&msg),
                    MOCK_OVERSEER_CONTRACT => self.handle_overseer_query(&msg),
                    MOCK_ORACLE_CONTRACT => self.handle_oracle_query(&msg),
                    MOCK_BLUNA_HUB_CONTRACT => self.handle_hub_query(&msg),
                    MOCK_AUST_TOKEN => self.handle_aust_query(&msg),
                    _ => panic!("Unsupported Query for address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
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
                    pending_rewards: Decimal256::zero(),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            MarketQueryMsg::Config {} => {
                let res = MarketConfigResponse {
                    owner_addr: "owner".to_string(),
                    aterra_contract: MOCK_AUST_TOKEN.to_owned(),
                    interest_model: "interest_model".to_string(),
                    distribution_model: "distribution_model".to_string(),
                    overseer_contract: MOCK_OVERSEER_CONTRACT.to_owned(),
                    collector_contract: "collector_contract".to_string(),
                    distributor_contract: "distributor_contract".to_string(),
                    stable_denom: "uusd".to_string(),
                    max_borrow_factor: Decimal256::one(),
                };

                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_custody_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            CustodyQueryMsg::Config {} => {
                let res = CustodyConfigResponse {
                    owner: "owner".to_string(),
                    collateral_token: MOCK_BLUNA_TOKEN.to_owned(),
                    overseer_contract: MOCK_OVERSEER_CONTRACT.to_owned(),
                    market_contract: MOCK_MARKET_CONTRACT.to_owned(),
                    reward_contract: "reward_contract".to_string(),
                    liquidation_contract: "liquidation_contract".to_string(),
                    stable_denom: "uusd".to_owned(),
                    basset_info: BAssetInfo {
                        name: "name".to_string(),
                        symbol: "symbol".to_string(),
                        decimals: 6,
                    },
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
            OverseerQueryMsg::Config {} => {
                let res = OverseerConfigResponse {
                    owner_addr: "owner".to_string(),
                    oracle_contract: MOCK_ORACLE_CONTRACT.to_owned(),
                    market_contract: MOCK_MARKET_CONTRACT.to_owned(),
                    liquidation_contract: "liquidiation_contract".to_string(),
                    collector_contract: "collector_contract".to_string(),
                    threshold_deposit_rate: Decimal256::one(),
                    target_deposit_rate: Decimal256::one(),
                    buffer_distribution_factor: Decimal256::one(),
                    anc_purchase_factor: Decimal256::one(),
                    stable_denom: "uusd".to_string(),
                    epoch_period: 0,
                    price_timeframe: 0,
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_hub_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            BLunaHubQueryMsg::State {} => {
                let res = BLunaHubStateResponse {
                    bluna_exchange_rate: Decimal::one(),
                    stluna_exchange_rate: Decimal::one(),
                    total_bond_bluna_amount: Uint128::zero(),
                    total_bond_stluna_amount: Uint128::zero(),
                    last_index_modification: 0,
                    prev_hub_balance: Uint128::zero(),
                    last_unbonded_time: 0,
                    last_processed_batch: 0,
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
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

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            token_balance: Uint128::zero(),
            loan_amount: Uint256::zero(),
        }
    }
}
