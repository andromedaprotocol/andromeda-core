use crate::{
    address_list::{IncludesAddressResponse, QueryMsg as AddressListQueryMsg},
    communication::hooks::{AndromedaHook, OnFundsTransferResponse},
    communication::AndromedaQuery,
    cw721::TransferAgreement,
    cw721::{
        MetadataAttribute, MetadataType, QueryMsg as Cw721QueryMsg, TokenExtension, TokenMetadata,
    },
    cw721_offers::{ExecuteMsg as OffersExecuteMsg, OfferResponse, QueryMsg as OffersQueryMsg},
    factory::{CodeIdResponse, QueryMsg as FactoryQueryMsg},
    ownership::ContractOwnerResponse,
    primitive::{GetValueResponse, Primitive, QueryMsg as PrimitiveQueryMsg},
    rates::{Funds, QueryMsg as RatesQueryMsg},
    receipt::{generate_receipt_message, QueryMsg as ReceiptQueryMsg},
};
use astroport::{
    asset::{AssetInfo, PairInfo},
    factory::{PairType, QueryMsg as AstroportFactoryQueryMsg},
    router::{QueryMsg as AstroportRouterQueryMsg, SimulateSwapOperationsResponse},
};
use cosmwasm_std::{
    coin, coins, from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, BankMsg, Binary, Coin, ContractResult, CosmosMsg, Decimal, Event, OwnedDeps,
    Querier, QuerierResult, QueryRequest, Response, SubMsg, SystemError, SystemResult, Uint128,
    WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg};

use cw721::{Expiration, NftInfoResponse, OwnerOfResponse};
use std::collections::HashMap;
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};

pub const MOCK_FACTORY_CONTRACT: &str = "factory_contract";
pub const MOCK_CW721_CONTRACT: &str = "cw721_contract";
pub const MOCK_ASTROPORT_WRAPPER_CONTRACT: &str = "astroport_wrapper_contract";
pub const MOCK_ASTROPORT_FACTORY_CONTRACT: &str = "astroport_factory_contract";
pub const MOCK_ASTROPORT_ROUTER_CONTRACT: &str = "astroport_router_contract";
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
pub const MOCK_CW20_CONTRACT: &str = "cw20_contract";
pub const MOCK_CW20_CONTRACT2: &str = "cw20_contract2";
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";
pub const MOCK_ADDRESSLIST_CONTRACT: &str = "addresslist_contract";
pub const MOCK_RECEIPT_CONTRACT: &str = "receipt_contract";
pub const MOCK_OFFERS_CONTRACT: &str = "offers_contract";

pub const MOCK_RATES_RECIPIENT: &str = "rates_recipient";
pub const MOCK_TOKEN_TRANSFER_AGREEMENT: &str = "token_transfer_agreement";

pub fn bank_sub_msg(amount: u128, recipient: &str) -> SubMsg {
    SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_owned(),
        amount: coins(amount, "uusd"),
    }))
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

pub struct WasmMockQuerier {
    pub base: MockQuerier<TerraQueryWrapper>,
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
                    MOCK_CW20_CONTRACT2 => self.handle_cw20_query(msg),
                    MOCK_CW721_CONTRACT => self.handle_cw721_query(msg),
                    MOCK_PRIMITIVE_CONTRACT => self.handle_primitive_query(msg),
                    MOCK_ASTROPORT_FACTORY_CONTRACT => self.handle_astroport_factory_query(msg),
                    MOCK_ASTROPORT_ROUTER_CONTRACT => self.handle_astroport_router_query(msg),
                    MOCK_RATES_CONTRACT => self.handle_rates_query(msg),
                    MOCK_ADDRESSLIST_CONTRACT => self.handle_addresslist_query(msg),
                    MOCK_OFFERS_CONTRACT => self.handle_offers_query(msg),
                    MOCK_RECEIPT_CONTRACT => self.handle_receipt_query(msg),
                    MOCK_FACTORY_CONTRACT => self.handle_factory_query(msg),
                    _ => {
                        let msg_response = IncludesAddressResponse { included: false };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
                    }
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_factory_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            FactoryQueryMsg::AndrQuery(AndromedaQuery::Get(data)) => {
                let key: String = from_binary(&data.unwrap()).unwrap();
                let code_id = match key.as_str() {
                    "receipt" => 1,
                    "rates" => 2,
                    "address_list" => 3,
                    "cw721" => 4,
                    _ => 0,
                };
                let response = CodeIdResponse { code_id };
                SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
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

    fn handle_rates_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            RatesQueryMsg::AndrHook(hook_msg) => match hook_msg {
                AndromedaHook::OnFundsTransfer {
                    sender: _,
                    payload: _,
                    amount,
                } => {
                    // Hardcodes a royalty of 10% and tax of 10%.
                    let (new_funds, msgs): (Funds, Vec<SubMsg>) = match amount {
                        Funds::Cw20(ref coin) => (
                            Funds::Cw20(Cw20Coin {
                                // Deduct royalty.
                                amount: coin.amount.multiply_ratio(90u128, 100u128),
                                address: coin.address.clone(),
                            }),
                            vec![self.get_cw20_rates_msg(coin), self.get_cw20_rates_msg(coin)],
                        ),
                        Funds::Native(ref coin) => (
                            Funds::Native(Coin {
                                // Deduct royalty.
                                amount: coin.amount.multiply_ratio(90u128, 100u128),
                                denom: coin.denom.clone(),
                            }),
                            vec![
                                self.get_native_rates_msg(coin, 10, None),
                                self.get_native_rates_msg(coin, 10, None),
                            ],
                        ),
                    };
                    let response = OnFundsTransferResponse {
                        msgs,
                        events: vec![Event::new("Royalty"), Event::new("Tax")],
                        leftover_funds: new_funds,
                    };
                    SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
                }
                _ => SystemResult::Ok(ContractResult::Err("UnsupportedOperation".to_string())),
            },

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
                    SystemResult::Ok(ContractResult::Err("Does not exist".to_string()))
                } else if let AssetInfo::NativeToken { denom } = asset_infos[0].clone() {
                    if denom == "uusd" {
                        let res = PairInfo {
                            asset_infos,
                            contract_addr: Addr::unchecked("addr"),
                            liquidity_token: Addr::unchecked("addr"),
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
                            contract_addr: Addr::unchecked("addr"),
                            liquidity_token: Addr::unchecked("addr"),
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

    fn handle_offers_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            OffersQueryMsg::AndrHook(msg) => match msg {
                AndromedaHook::OnTransfer {
                    recipient,
                    token_id,
                    sender,
                } => {
                    if recipient == "purchaser" {
                        let msg: SubMsg = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: MOCK_OFFERS_CONTRACT.to_owned(),
                            funds: vec![],
                            msg: to_binary(&OffersExecuteMsg::AcceptOffer {
                                token_id,
                                recipient: sender,
                            })
                            .unwrap(),
                        }));
                        let resp = Response::new().add_submessage(msg);
                        return SystemResult::Ok(ContractResult::Ok(to_binary(&resp).unwrap()));
                    }
                    panic!("Unsupported Query")
                }
                _ => SystemResult::Ok(ContractResult::Err("UnsupportedOperation".to_string())),
            },
            OffersQueryMsg::Offer { .. } => {
                let response = OfferResponse {
                    denom: "uusd".to_string(),
                    offer_amount: Uint128::zero(),
                    tax_amount: Uint128::zero(),
                    remaining_amount: Uint128::zero(),
                    expiration: Expiration::Never {},
                    purchaser: "purchaser".to_string(),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_addresslist_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            AddressListQueryMsg::AndrHook(hook_msg) => match hook_msg {
                AndromedaHook::OnExecute { sender, payload: _ } => {
                    let whitelisted_addresses = ["sender", "minter", "purchaser", "creator"];
                    let response: Response = Response::default();
                    if whitelisted_addresses.contains(&sender.as_str()) {
                        SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
                    } else {
                        SystemResult::Ok(ContractResult::Err("InvalidAddress".to_string()))
                    }
                }
                _ => SystemResult::Ok(ContractResult::Err("UnsupportedOperation".to_string())),
            },
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_receipt_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            ReceiptQueryMsg::AndrHook(hook_msg) => match hook_msg {
                AndromedaHook::OnFundsTransfer {
                    sender: _,
                    payload,
                    amount,
                } => {
                    let events: Vec<Event> = from_binary(&payload).unwrap();
                    let receipt_msg =
                        generate_receipt_message(MOCK_RECEIPT_CONTRACT.into(), events).unwrap();
                    let response = OnFundsTransferResponse {
                        msgs: vec![receipt_msg],
                        events: vec![],
                        leftover_funds: amount,
                    };
                    SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
                }
                _ => SystemResult::Ok(ContractResult::Err("UnsupportedOperation".to_string())),
            },
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_cw721_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw721QueryMsg::NftInfo { token_id } => {
                let transfer_agreement = if token_id == MOCK_TOKEN_TRANSFER_AGREEMENT {
                    Some(TransferAgreement {
                        amount: coin(100, "uusd"),
                        purchaser: "purchaser".to_string(),
                    })
                } else {
                    None
                };
                let extension = if token_id == "original_token_id" {
                    TokenExtension {
                        name: "wrapped_token_id".to_owned(),
                        publisher: "sender".to_owned(),
                        description: None,
                        transfer_agreement: None,
                        metadata: Some(TokenMetadata {
                            data_type: MetadataType::Other,
                            external_url: None,
                            data_url: None,
                            attributes: Some(vec![
                                MetadataAttribute {
                                    key: "original_token_id".to_owned(),
                                    value: "original_token_id".to_owned(),
                                    display_label: None,
                                },
                                MetadataAttribute {
                                    key: "original_token_address".to_owned(),
                                    value: "original_token_address".to_owned(),
                                    display_label: None,
                                },
                            ]),
                        }),
                        archived: false,
                        pricing: None,
                    }
                } else {
                    TokenExtension {
                        name: token_id,
                        publisher: "sender".to_owned(),
                        description: None,
                        transfer_agreement,
                        metadata: None,
                        archived: false,
                        pricing: None,
                    }
                };
                let response = NftInfoResponse {
                    token_uri: None,
                    extension,
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
            }
            Cw721QueryMsg::OwnerOf { .. } => {
                let response = OwnerOfResponse {
                    owner: "creator".to_string(),
                    approvals: vec![],
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
            }
            Cw721QueryMsg::AndrHook(AndromedaHook::OnFundsTransfer { amount, .. }) => {
                let c = amount.try_get_coin().unwrap();
                let response = OnFundsTransferResponse {
                    events: vec![Event::new("Royalty"), Event::new("Tax")],
                    leftover_funds: Funds::Native(coin(
                        c.amount.multiply_ratio(90u128, 100u128).u128(),
                        c.denom.clone(),
                    )),
                    msgs: vec![
                        // 10% tax message.
                        self.get_native_rates_msg(&c, 10, None),
                        // 10% royalty message.
                        self.get_native_rates_msg(&c, 10, None),
                    ],
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
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
            PrimitiveQueryMsg::AndrQuery(AndromedaQuery::Get(data)) => {
                let name: String = from_binary(&data.unwrap()).unwrap();
                let msg_response = match name.as_str() {
                    "percent" => GetValueResponse {
                        name,
                        value: Primitive::Uint128(1u128.into()),
                    },
                    "flat" => GetValueResponse {
                        name,
                        value: Primitive::Coin(coin(1u128, "uusd")),
                    },
                    "flat_cw20" => GetValueResponse {
                        name,
                        value: Primitive::Coin(coin(1u128, "address")),
                    },
                    "factory" => GetValueResponse {
                        name,
                        value: Primitive::String(MOCK_FACTORY_CONTRACT.to_owned()),
                    },
                    _ => panic!("Unsupported primitive name"),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn get_native_rates_msg(
        &self,
        coin: &Coin,
        numerator: u128,
        recipient: Option<String>,
    ) -> SubMsg {
        let recipient = recipient.unwrap_or_else(|| MOCK_RATES_RECIPIENT.to_string());
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient,
            amount: vec![Coin {
                amount: coin.amount.multiply_ratio(numerator, 100u128),
                denom: coin.denom.clone(),
            }],
        }))
    }

    fn get_cw20_rates_msg(&self, coin: &Cw20Coin) -> SubMsg {
        SubMsg::new(WasmMsg::Execute {
            contract_addr: MOCK_CW20_CONTRACT.into(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: MOCK_RATES_RECIPIENT.to_string(),
                amount: coin.amount.multiply_ratio(10u128, 100u128),
            })
            .unwrap(),
            funds: vec![],
        })
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
