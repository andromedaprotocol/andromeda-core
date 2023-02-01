use common::{
    ado_base::{
        hooks::{AndromedaHook, OnFundsTransferResponse},
        ownership::ContractOwnerResponse,
        AndromedaQuery, QueryMsg,
    },
    primitive::{GetValueResponse, Primitive, Value},
    Funds,
};

use andromeda_app::app::QueryMsg as MissionQueryMsg;
use andromeda_modules::{
    address_list::{IncludesAddressResponse, QueryMsg as AddressListQueryMsg},
    rates::QueryMsg as RatesQueryMsg,
    receipt::{generate_receipt_message, QueryMsg as ReceiptQueryMsg},
};
use andromeda_non_fungible_tokens::{
    cw721::{MetadataAttribute, QueryMsg as Cw721QueryMsg, TokenExtension, TransferAgreement},
    cw721_bid::{BidResponse, ExecuteMsg as BidsExecuteMsg, QueryMsg as BidsQueryMsg},
};
use andromeda_os::adodb::QueryMsg as FactoryQueryMsg;
use cosmwasm_std::{
    coin, coins, from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, BankMsg, Binary, Coin, ContractResult, CosmosMsg, Decimal, Event, OwnedDeps,
    Querier, QuerierResult, QueryRequest, Response, SubMsg, SystemError, SystemResult, Uint128,
    WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg};

use cw721::{Expiration, NftInfoResponse, OwnerOfResponse};

pub const MOCK_FACTORY_CONTRACT: &str = "factory_contract";
pub const MOCK_CW721_CONTRACT: &str = "cw721_contract";
pub const MOCK_AUCTION_CONTRACT: &str = "auction_contract";
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
pub const MOCK_CW20_CONTRACT: &str = "cw20_contract";
pub const MOCK_CW20_CONTRACT2: &str = "cw20_contract2";
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";
pub const MOCK_ADDRESSLIST_CONTRACT: &str = "addresslist_contract";
pub const MOCK_RECEIPT_CONTRACT: &str = "receipt_contract";
pub const MOCK_APP_CONTRACT: &str = "app_contract";
pub const MOCK_BIDS_CONTRACT: &str = "bids_contract";

pub const MOCK_RATES_RECIPIENT: &str = "rates_recipient";
pub const MOCK_TOKEN_TRANSFER_AGREEMENT: &str = "token_transfer_agreement";
pub const MOCK_TOKEN_ARCHIVED: &str = "token_archived";

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
                    error: format!("Parsing query request: {e}"),
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
                    MOCK_RATES_CONTRACT => self.handle_rates_query(msg),
                    MOCK_ADDRESSLIST_CONTRACT => self.handle_addresslist_query(msg),
                    MOCK_BIDS_CONTRACT => self.handle_bids_query(msg),
                    MOCK_RECEIPT_CONTRACT => self.handle_receipt_query(msg),
                    MOCK_FACTORY_CONTRACT => self.handle_factory_query(msg),
                    MOCK_APP_CONTRACT => self.handle_app_query(msg),
                    _ => {
                        let msg_response = IncludesAddressResponse { included: false };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&msg_response).unwrap()))
                    }
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_app_query(&self, msg: &Binary) -> QuerierResult {
        let valid_identifiers = ["e", "b"];
        match from_binary(msg).unwrap() {
            MissionQueryMsg::ComponentExists { name } => {
                let value = valid_identifiers.contains(&name.as_str());
                SystemResult::Ok(ContractResult::Ok(to_binary(&value).unwrap()))
            }
            _ => panic!("Unsupported Query: {}", msg),
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
                    "swapper_impl" => 5,
                    _ => 0,
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&code_id).unwrap()))
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
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&None::<Response>).unwrap())),
            },

            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_bids_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            BidsQueryMsg::AndrHook(msg) => match msg {
                AndromedaHook::OnTransfer {
                    recipient,
                    token_id,
                    sender,
                } => {
                    if recipient == "purchaser" {
                        let msg: SubMsg = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: MOCK_BIDS_CONTRACT.to_owned(),
                            funds: vec![],
                            msg: to_binary(&BidsExecuteMsg::AcceptBid {
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
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&None::<Response>).unwrap())),
            },
            BidsQueryMsg::Bid { .. } => {
                let response = BidResponse {
                    denom: "uusd".to_string(),
                    bid_amount: Uint128::zero(),
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
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&None::<Response>).unwrap())),
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
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&None::<Response>).unwrap())),
            },
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_cw721_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw721QueryMsg::NftInfo { token_id } => {
                let extension = if token_id == "original_token_id" {
                    TokenExtension {
                        name: "wrapped_token_id".to_owned(),
                        publisher: "sender".to_owned(),
                        description: None,
                        attributes: vec![
                            MetadataAttribute {
                                trait_type: "original_token_id".to_owned(),
                                value: "original_token_id".to_owned(),
                                display_type: None,
                            },
                            MetadataAttribute {
                                trait_type: "original_token_address".to_owned(),
                                value: "original_token_address".to_owned(),
                                display_type: None,
                            },
                        ],
                        image: String::from(""),
                        image_data: None,
                        external_url: None,
                        animation_url: None,
                        youtube_url: None,
                    }
                } else {
                    TokenExtension {
                        name: token_id,
                        publisher: "sender".to_owned(),
                        description: None,
                        attributes: vec![],
                        image: String::from(""),
                        image_data: None,
                        external_url: None,
                        animation_url: None,
                        youtube_url: None,
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
            Cw721QueryMsg::IsArchived { token_id } => {
                if token_id == MOCK_TOKEN_ARCHIVED {
                    SystemResult::Ok(ContractResult::Ok(to_binary(&true).unwrap()))
                } else {
                    SystemResult::Ok(ContractResult::Ok(to_binary(&false).unwrap()))
                }
            }
            Cw721QueryMsg::TransferAgreement { token_id } => {
                if token_id == MOCK_TOKEN_TRANSFER_AGREEMENT {
                    SystemResult::Ok(ContractResult::Ok(
                        to_binary(&Some(TransferAgreement {
                            amount: Value::Raw(Coin {
                                denom: "uusd".to_string(),
                                amount: Uint128::from(10u64),
                            }),
                            purchaser: "purchaser".to_string(),
                        }))
                        .unwrap(),
                    ))
                } else {
                    let resp: Option<TransferAgreement> = None;
                    SystemResult::Ok(ContractResult::Ok(to_binary(&resp).unwrap()))
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
            QueryMsg::AndrQuery(AndromedaQuery::Get(data)) => {
                let key: String = from_binary(&data.unwrap()).unwrap();
                let msg_response = match key.as_str() {
                    "percent" => GetValueResponse {
                        key,
                        value: Primitive::Decimal(Decimal::percent(1)),
                    },
                    "flat" => GetValueResponse {
                        key,
                        value: Primitive::Coin(coin(1u128, "uusd")),
                    },
                    "flat_cw20" => GetValueResponse {
                        key,
                        value: Primitive::Coin(coin(1u128, "address")),
                    },
                    "sell_amount" => GetValueResponse {
                        key,
                        value: Primitive::Coin(coin(100, "uusd")),
                    },
                    "adodb" => GetValueResponse {
                        key,
                        value: Primitive::String(MOCK_FACTORY_CONTRACT.to_owned()),
                    },
                    _ => panic!("Unsupported primitive key"),
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

    pub fn new(base: MockQuerier<cosmwasm_std::Empty>) -> Self {
        WasmMockQuerier { base }
    }
}
