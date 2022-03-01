use astroport::{
    asset::{Asset, AssetInfo, PairInfo},
    factory::{ConfigResponse, PairType, QueryMsg as AstroportFactoryQueryMsg},
    generator::{PendingTokenResponse, QueryMsg as GeneratorQueryMsg},
    pair::QueryMsg as AstroportPairQueryMsg,
    router::{QueryMsg as AstroportRouterQueryMsg, SimulateSwapOperationsResponse},
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, BalanceResponse as NativeBalanceResponse, BankQuery, Binary, Coin,
    ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult,
    Uint128, WasmQuery,
};
use cw20::{BalanceResponse, Cw20QueryMsg};

use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_ASTROPORT_PAIR_CONTRACT: &str = "astroport_pair_contract";
pub const MOCK_ASTROPORT_FACTORY_CONTRACT: &str = "astroport_factory_contract";
pub const MOCK_ASTROPORT_GENERATOR_CONTRACT: &str = "astroport_generator_contract";
pub const MOCK_ASTROPORT_ROUTER_CONTRACT: &str = "astroport_router_contract";
pub const MOCK_LP_ASSET1: &str = "token1";
pub const MOCK_LP_ASSET2: &str = "token2";
pub const MOCK_LP_TOKEN_CONTRACT: &str = "lp_token_contract";

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
    pub base: MockQuerier<TerraQueryWrapper>,
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
            QueryRequest::Bank(BankQuery::Balance { address, denom }) => {
                if address == MOCK_ASTROPORT_PAIR_CONTRACT {
                    let res = NativeBalanceResponse {
                        amount: Coin {
                            denom: denom.to_owned(),
                            amount: 10u128.into(),
                        },
                    };
                    SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                } else {
                    self.base.handle_query(request)
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_LP_ASSET1 => self.handle_lp_asset1_query(msg),
                    MOCK_LP_ASSET2 => self.handle_lp_asset2_query(msg),
                    MOCK_LP_TOKEN_CONTRACT => self.handle_lp_token_query(msg),
                    MOCK_ASTROPORT_FACTORY_CONTRACT => self.handle_astroport_factory_query(msg),
                    MOCK_ASTROPORT_GENERATOR_CONTRACT => self.handle_astroport_generator_query(msg),
                    MOCK_ASTROPORT_PAIR_CONTRACT => self.handle_astroport_pair_query(msg),
                    MOCK_ASTROPORT_ROUTER_CONTRACT => self.handle_astroport_router_query(msg),
                    _ => {
                        panic!("Unsupported Query for  {}", contract_addr)
                    }
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_astroport_generator_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            GeneratorQueryMsg::PendingToken { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&Uint128::from(10u128)).unwrap(),
            )),
            GeneratorQueryMsg::Deposit { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&Uint128::from(10u128)).unwrap(),
            )),
            _ => panic!("Unsupported query"),
        }
    }

    fn handle_astroport_pair_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            AstroportPairQueryMsg::Pair {} => {
                let res = PairInfo {
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: Addr::unchecked("token1"),
                        },
                        AssetInfo::Token {
                            contract_addr: Addr::unchecked("token2"),
                        },
                    ],
                    contract_addr: Addr::unchecked(MOCK_ASTROPORT_PAIR_CONTRACT),
                    liquidity_token: Addr::unchecked(MOCK_LP_TOKEN_CONTRACT),
                    pair_type: PairType::Xyk {},
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            AstroportPairQueryMsg::Share { .. } => {
                let res = vec![
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: Addr::unchecked("token1"),
                        },
                        amount: 10u128.into(),
                    },
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: Addr::unchecked("token2"),
                        },
                        amount: 20u128.into(),
                    },
                ];
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_astroport_router_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            AstroportRouterQueryMsg::SimulateSwapOperations { .. } => {
                let res = SimulateSwapOperationsResponse {
                    amount: Uint128::zero(),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_astroport_factory_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            AstroportFactoryQueryMsg::Config {} => {
                let res = ConfigResponse {
                    owner: Addr::unchecked("owner"),
                    pair_configs: vec![],
                    token_code_id: 1,
                    fee_address: None,
                    generator_address: Some(Addr::unchecked(MOCK_ASTROPORT_GENERATOR_CONTRACT)),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            AstroportFactoryQueryMsg::Pair { asset_infos } => {
                if matches!(
                    asset_infos,
                    [AssetInfo::NativeToken { .. }, AssetInfo::NativeToken { .. }]
                ) {
                    SystemResult::Ok(ContractResult::Err("Does not exist".to_string()))
                } else if let AssetInfo::NativeToken { denom } = asset_infos[0].clone() {
                    if denom == "uusd" {
                        let res = PairInfo {
                            asset_infos,
                            contract_addr: Addr::unchecked(MOCK_ASTROPORT_PAIR_CONTRACT),
                            liquidity_token: Addr::unchecked(MOCK_LP_TOKEN_CONTRACT),
                            pair_type: PairType::Xyk {},
                        };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                    } else {
                        SystemResult::Ok(ContractResult::Err("Does not exist".to_string()))
                    }
                } else if let AssetInfo::NativeToken { denom } = asset_infos[1].clone() {
                    if denom == "uusd" {
                        let res = PairInfo {
                            asset_infos,
                            contract_addr: Addr::unchecked(MOCK_ASTROPORT_PAIR_CONTRACT),
                            liquidity_token: Addr::unchecked(MOCK_LP_TOKEN_CONTRACT),
                            pair_type: PairType::Xyk {},
                        };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                    } else {
                        SystemResult::Ok(ContractResult::Err("Does not exist".to_string()))
                    }
                } else if let [AssetInfo::Token {
                    contract_addr: contract_addr1,
                }, AssetInfo::Token {
                    contract_addr: contract_addr2,
                }] = asset_infos.clone()
                {
                    if contract_addr1 == MOCK_LP_ASSET1 && contract_addr2 == MOCK_LP_ASSET2 {
                        let res = PairInfo {
                            asset_infos,
                            contract_addr: Addr::unchecked(MOCK_ASTROPORT_PAIR_CONTRACT),
                            liquidity_token: Addr::unchecked(MOCK_LP_TOKEN_CONTRACT),
                            pair_type: PairType::Xyk {},
                        };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                    } else {
                        SystemResult::Ok(ContractResult::Err("Does not exist".to_string()))
                    }
                } else {
                    SystemResult::Ok(ContractResult::Err("Does not exist".to_string()))
                }
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_lp_asset1_query(&self, msg: &Binary) -> QuerierResult {
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

    fn handle_lp_asset2_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw20QueryMsg::Balance { .. } => {
                let balance_response = BalanceResponse {
                    balance: 40u128.into(),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&balance_response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_lp_token_query(&self, msg: &Binary) -> QuerierResult {
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

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier { base }
    }
}
