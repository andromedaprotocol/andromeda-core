use andromeda_app::app::QueryMsg as AppQueryMsg;
use common::{ado_base::hooks::RatesResponse, Funds};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_json_binary, BankMsg, Binary, Coin, ContractResult, CosmosMsg, OwnedDeps, Querier,
    QuerierResult, QueryRequest, Response, SubMsg, SystemError, SystemResult, Uint128, WasmQuery,
};

pub const MOCK_TOKEN_CONTRACT: &str = "token_contract";
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";
pub const MOCK_WEIGHTED_DISTRIBUTION_SPLITTER_CONTRACT: &str =
    "weighted_distribution_splitter_contract";
pub const MOCK_APP_CONTRACT: &str = "app_contract";
pub const MOCK_ADDRESSLIST_CONTRACT: &str = "addresslist_contract";
pub const MOCK_RECIPIENT1: &str = "mock_recipient1";
pub const MOCK_RECIPIENT2: &str = "mock_recipient2";

pub const MOCK_TOKEN_ADDR: &str = "token0001";
pub const MOCK_TOKEN_OWNER: &str = "owner";
pub const MOCK_UNCLAIMED_TOKEN: &str = "unclaimed_token";
pub const MOCK_TAX_RECIPIENT: &str = "tax_recipient";
pub const MOCK_ROYALTY_RECIPIENT: &str = "royalty_recipient";
pub const MOCK_TOKENS_FOR_SALE: &[&str] = &[
    "token1", "token2", "token3", "token4", "token5", "token6", "token7",
];

pub const MOCK_CONDITIONS_MET_CONTRACT: &str = "conditions_met";
pub const MOCK_CONDITIONS_NOT_MET_CONTRACT: &str = "conditions_not_met";

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
    pub contract_address: String,
    pub tokens_left_to_burn: usize,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<cosmwasm_std::Empty> = match from_json(bin_request) {
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
                    MOCK_WEIGHTED_DISTRIBUTION_SPLITTER_CONTRACT => todo!(),
                    MOCK_RATES_CONTRACT => self.handle_rates_query(msg),
                    MOCK_APP_CONTRACT => self.handle_app_query(msg),
                    MOCK_ADDRESSLIST_CONTRACT => self.handle_addresslist_query(msg),
                    _ => panic!("Unknown Contract Address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_app_query(&self, msg: &Binary) -> QuerierResult {
        let valid_identifiers = ["e", "b"];
        match from_json(msg).unwrap() {
            AppQueryMsg::ComponentExists { name } => {
                let value = valid_identifiers.contains(&name.as_str());
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&value).unwrap()))
            }
            _ => panic!("Unsupported Query: {}", msg),
        }
    }

    fn handle_rates_query(&self, msg: &Binary) -> QuerierResult {
        match from_json(msg).unwrap() {
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
                                    to_address: MOCK_ROYALTY_RECIPIENT.to_owned(),
                                    amount: vec![Coin {
                                        // Royalty of 10%
                                        amount: coin.amount.multiply_ratio(10u128, 100u128),
                                        denom: coin.denom.clone(),
                                    }],
                                })),
                                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                                    to_address: MOCK_TAX_RECIPIENT.to_owned(),
                                    amount: vec![Coin {
                                        // Flat tax of 50
                                        amount: Uint128::from(50u128),
                                        denom: coin.denom.clone(),
                                    }],
                                })),
                            ],
                        ),
                        Funds::Cw20(_) => {
                            return SystemResult::Ok(ContractResult::Ok(
                                to_json_binary(&None::<Response>).unwrap(),
                            ))
                        }
                    };
                    let response = RatesResponse {
                        msgs,
                        events: vec![],
                        leftover_funds: new_funds,
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&response).unwrap()))
                }
                _ => SystemResult::Ok(ContractResult::Ok(
                    to_json_binary(&None::<Response>).unwrap(),
                )),
            },
        }
    }

    fn handle_addresslist_query(&self, msg: &Binary) -> QuerierResult {
        match from_json(msg).unwrap() {
            HookMsg::AndrHook(hook_msg) => match hook_msg {
                AndromedaHook::OnExecute { sender, payload: _ } => {
                    let whitelisted_addresses = ["sender"];
                    let response: Response = Response::default();
                    if whitelisted_addresses.contains(&sender.as_str()) {
                        SystemResult::Ok(ContractResult::Ok(to_json_binary(&response).unwrap()))
                    } else {
                        SystemResult::Ok(ContractResult::Err("InvalidAddress".to_string()))
                    }
                }
                _ => SystemResult::Ok(ContractResult::Err("UnsupportedOperation".to_string())),
            },
        }
    }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            contract_address: mock_env().contract.address.to_string(),
            tokens_left_to_burn: 2,
        }
    }
}
