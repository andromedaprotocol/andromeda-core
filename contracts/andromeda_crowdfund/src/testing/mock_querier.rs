use common::{
    ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse},
    Funds,
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, BankMsg, Binary, Coin, ContractResult, CosmosMsg, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SubMsg, SystemError, SystemResult, Uint128, WasmQuery,
};
use cw721::{Cw721QueryMsg, OwnerOfResponse, TokensResponse};
use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_TOKEN_CONTRACT: &str = "token_contract";
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";

pub const MOCK_RATES_RECIPIENT: &str = "rates_recipient";
pub const MOCK_NON_EXISTING_TOKEN: &str = "non_existing_token";
pub const MOCK_TOKENS_FOR_SALE: &[&str] = &["token1", "token2", "token3", "token4", "token5"];

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
                    MOCK_TOKEN_CONTRACT => self.handle_token_query(msg),
                    MOCK_RATES_CONTRACT => self.handle_rates_query(msg),
                    _ => panic!("Unknown Contract Address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_token_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            Cw721QueryMsg::Tokens { .. } => {
                let res = TokensResponse {
                    tokens: MOCK_TOKENS_FOR_SALE
                        .to_vec()
                        .into_iter()
                        .map(String::from)
                        .collect(),
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
            }
            Cw721QueryMsg::OwnerOf { token_id, .. } => {
                if token_id == MOCK_NON_EXISTING_TOKEN {
                    return SystemResult::Ok(ContractResult::Err("error".to_string()));
                } else if MOCK_TOKENS_FOR_SALE.contains(&token_id.as_str()) {
                    let res = OwnerOfResponse {
                        owner: mock_env().contract.address.to_string(),
                        approvals: vec![],
                    };
                    return SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()));
                } else {
                    let res = OwnerOfResponse {
                        owner: "not_contract".to_string(),
                        approvals: vec![],
                    };
                    return SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()));
                }
            }

            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_rates_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            HookMsg::AndrHook(hook_msg) => match hook_msg {
                AndromedaHook::OnFundsTransfer {
                    sender: _,
                    payload: _,
                    amount,
                } => {
                    let (new_funds, msgs): (Funds, Vec<SubMsg>) = match amount {
                        Funds::Native(ref coin) => (
                            Funds::Native(Coin {
                                // Deduct royalty of 10%.
                                amount: coin.amount.multiply_ratio(90u128, 100u128),
                                denom: coin.denom.clone(),
                            }),
                            vec![
                                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                                    to_address: MOCK_RATES_RECIPIENT.to_owned(),
                                    amount: vec![Coin {
                                        // Royalty of 10%
                                        amount: coin.amount.multiply_ratio(10u128, 100u128),
                                        denom: coin.denom.clone(),
                                    }],
                                })),
                                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                                    to_address: MOCK_RATES_RECIPIENT.to_owned(),
                                    amount: vec![Coin {
                                        // Flat tax of 50
                                        amount: Uint128::from(50u128),
                                        denom: coin.denom.clone(),
                                    }],
                                })),
                            ],
                        ),
                        Funds::Cw20(_) => {
                            return SystemResult::Ok(ContractResult::Err(
                                "UnsupportedOperation".to_string(),
                            ))
                        }
                    };
                    let response = OnFundsTransferResponse {
                        msgs,
                        events: vec![],
                        leftover_funds: new_funds,
                    };
                    SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
                }
                _ => SystemResult::Ok(ContractResult::Err("UnsupportedOperation".to_string())),
            },
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier { base }
    }
}
