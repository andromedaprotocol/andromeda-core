use common::{
    ado_base::hooks::{AndromedaHook, OnFundsTransferResponse},
    Funds,
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, BankMsg, Binary, Coin, ContractResult, CosmosMsg, Event, OwnedDeps, Querier,
    QuerierResult, QueryRequest, Response, SubMsg, SystemError, SystemResult, WasmMsg, WasmQuery,
};
use cw721::{Cw721QueryMsg, OwnerOfResponse};

use andromeda_modules::rates::QueryMsg as RatesQueryMsg;
use cw20::{Cw20Coin, Cw20ExecuteMsg};

pub const MOCK_TOKEN_ADDR: &str = "token0001";
pub const MOCK_TOKEN_OWNER: &str = "owner";
pub const MOCK_UNCLAIMED_TOKEN: &str = "unclaimed_token";
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";
pub const MOCK_CW20_CONTRACT: &str = "cw20_contract";
pub const MOCK_RATES_RECIPIENT: &str = "rates_recipient";

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
    base: MockQuerier,
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
                    MOCK_TOKEN_ADDR => self.handle_token_query(msg),
                    MOCK_RATES_CONTRACT => self.handle_rates_query(msg),
                    _ => panic!("Unknown Contract Address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_token_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw721QueryMsg::OwnerOf { token_id, .. } => {
                let res = if token_id == MOCK_UNCLAIMED_TOKEN {
                    OwnerOfResponse {
                        owner: mock_env().contract.address.to_string(),
                        approvals: vec![],
                    }
                } else {
                    OwnerOfResponse {
                        owner: MOCK_TOKEN_OWNER.to_owned(),
                        approvals: vec![],
                    }
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
                    SystemResult::Ok(ContractResult::Ok(to_binary(&Some(response)).unwrap()))
                }
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&None::<Response>).unwrap())),
            },

            _ => panic!("Unsupported Query"),
        }
    }

    pub fn new(base: MockQuerier<cosmwasm_std::Empty>) -> Self {
        WasmMockQuerier { base }
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
}
