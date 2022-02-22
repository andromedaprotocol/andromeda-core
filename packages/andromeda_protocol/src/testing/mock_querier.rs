use crate::{
    address_list::IncludesAddressResponse,
    auction::{AuctionStateResponse, QueryMsg as AuctionQueryMsg},
    ownership::ContractOwnerResponse,
    primitive::{GetValueResponse, Primitive, QueryMsg as PrimitiveQueryMsg},
};
use astroport::{
    asset::{AssetInfo, PairInfo},
    factory::{PairType, QueryMsg as AstroportFactoryQueryMsg},
    router::{QueryMsg as AstroportRouterQueryMsg, SimulateSwapOperationsResponse},
};
use cosmwasm_std::{
    coin, from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, Binary, Coin, ContractResult, Decimal, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, Timestamp, Uint128, WasmQuery,
};
use cw20::{BalanceResponse, Cw20QueryMsg};

use cw721::Expiration;
use std::collections::HashMap;
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};

pub const MOCK_AUCTION_CONTRACT: &str = "auction_contract";
pub const MOCK_ASTROPORT_FACTORY_CONTRACT: &str = "astroport_factory_contract";
pub const MOCK_ASTROPORT_ROUTER_CONTRACT: &str = "astroport_router_contract";
pub const MOCK_TOKEN_IN_AUCTION: &str = "token1";
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
pub const MOCK_CW20_CONTRACT: &str = "cw20_contract";

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
    tax_querier: TaxQuerier,
}

#[derive(Clone, Default)]
pub struct TaxQuerier {
    rate: Decimal,
    // this lets us iterate over all pairs that match the first string
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
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
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                if &TerraRoute::Treasury == route {
                    match query_data {
                        TerraQuery::TaxRate {} => {
                            let res = TaxRateResponse {
                                rate: self.tax_querier.rate,
                            };
                            SystemResult::Ok(ContractResult::from(to_binary(&res)))
                        }
                        TerraQuery::TaxCap { denom } => {
                            let cap = self
                                .tax_querier
                                .caps
                                .get(denom)
                                .copied()
                                .unwrap_or_default();
                            let res = TaxCapResponse { cap };
                            SystemResult::Ok(ContractResult::from(to_binary(&res)))
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    "addresslist_contract_address1" => {
                        let msg_response = IncludesAddressResponse { included: true };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
                    }
                    "factory_address" => {
                        let msg_response = ContractOwnerResponse {
                            owner: String::from("creator"),
                        };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
                    }
                    MOCK_CW20_CONTRACT => self.handle_cw20_query(msg),
                    MOCK_PRIMITIVE_CONTRACT => self.handle_primitive_query(msg),
                    MOCK_ASTROPORT_FACTORY_CONTRACT => self.handle_astroport_factory_query(msg),
                    MOCK_ASTROPORT_ROUTER_CONTRACT => self.handle_astroport_router_query(msg),
                    MOCK_AUCTION_CONTRACT => self.handle_auction_query(msg),
                    _ => {
                        let msg_response = IncludesAddressResponse { included: false };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
                    }
                }
            }
            _ => self.base.handle_query(request),
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
            AstroportFactoryQueryMsg::Pair { asset_infos } => {
                if matches!(
                    asset_infos,
                    [AssetInfo::NativeToken { .. }, AssetInfo::NativeToken { .. }]
                ) {
                    return SystemResult::Ok(ContractResult::Err("Does not exist".to_string()));
                } else if let AssetInfo::NativeToken { denom } = asset_infos[0].clone() {
                    if denom == "uusd" {
                        let res = PairInfo {
                            asset_infos,
                            contract_addr: Addr::unchecked("addr"),
                            liquidity_token: Addr::unchecked("addr"),
                            pair_type: PairType::Xyk {},
                        };
                        return SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()));
                    } else {
                        return SystemResult::Ok(ContractResult::Err("Does not exist".to_string()));
                    }
                } else {
                    panic!("Unsupported Query")
                }
            }
            _ => panic!("Unsupported Query"),
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
            PrimitiveQueryMsg::GetValue { name } => {
                let msg_response = match name.clone().unwrap().as_str() {
                    "percent" => GetValueResponse {
                        name: name.unwrap(),
                        value: Primitive::Uint128(1u128.into()),
                    },
                    "flat" => GetValueResponse {
                        name: name.unwrap(),
                        value: Primitive::Coin(coin(1u128, "uusd")),
                    },
                    "flat_cw20" => GetValueResponse {
                        name: name.unwrap(),
                        value: Primitive::Coin(coin(1u128, "address")),
                    },
                    _ => panic!("Unsupported rate name"),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_auction_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            AuctionQueryMsg::LatestAuctionState { token_id } => {
                let mut res = AuctionStateResponse {
                    start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
                    end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
                    high_bidder_addr: "address".to_string(),
                    high_bidder_amount: Uint128::from(100u128),
                    auction_id: Uint128::zero(),
                    coin_denom: "uusd".to_string(),
                    claimed: true,
                    whitelist: None,
                };
                if token_id == MOCK_TOKEN_IN_AUCTION {
                    res.claimed = false;
                }
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            tax_querier: TaxQuerier::default(),
        }
    }

    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }
}
