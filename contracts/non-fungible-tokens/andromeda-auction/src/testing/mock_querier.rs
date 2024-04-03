use andromeda_std::ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse};
use andromeda_std::ado_base::InstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::Funds;
use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
use cosmwasm_schema::cw_serde;

use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{coin, BankMsg, BankQuery, CosmosMsg, QuerierWrapper, Response, SubMsg};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_json_binary, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};
use cw721::{Cw721QueryMsg, OwnerOfResponse, TokensResponse};

pub use andromeda_std::testing::mock_querier::{
    MOCK_ADDRESS_LIST_CONTRACT, MOCK_APP_CONTRACT, MOCK_KERNEL_CONTRACT, MOCK_RATES_CONTRACT,
};

pub const MOCK_TOKEN_CONTRACT: &str = "token_contract";
pub const MOCK_UNCLAIMED_TOKEN: &str = "unclaimed_token";
pub const MOCK_TOKEN_ADDR: &str = "token_addr";
pub const MOCK_CW20_ADDR: &str = "cw20_addr";
pub const MOCK_RATES_RECIPIENT: &str = "rates_recipient";
pub const MOCK_TOKEN_OWNER: &str = "owner";
pub const MOCK_TOKENS_FOR_SALE: &[&str] = &[
    "token1", "token2", "token3", "token4", "token5", "token6", "token7",
];

pub const MOCK_CONDITIONS_MET_CONTRACT: &str = "conditions_met";
pub const MOCK_CONDITIONS_NOT_MET_CONTRACT: &str = "conditions_not_met";

/// Alternative to `cosmwasm_std::testing::mock_dependencies` that allows us to respond to custom queries.
///
/// Automatically assigns a kernel address as MOCK_KERNEL_CONTRACT.
pub fn mock_dependencies_custom(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));
    let storage = MockStorage::default();
    let mut deps = OwnedDeps {
        storage,
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: std::marker::PhantomData,
    };
    ADOContract::default()
        .instantiate(
            &mut deps.storage,
            mock_env(),
            &deps.api,
            &QuerierWrapper::new(&deps.querier),
            mock_info("sender", &[]),
            InstantiateMsg {
                ado_type: "crowdfund".to_string(),
                ado_version: "test".to_string(),

                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
                owner: None,
            },
        )
        .unwrap();
    deps
}

pub struct WasmMockQuerier {
    pub base: MockQuerier,
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

// NOTE: It's impossible to construct a non_exhaustive struct from another another crate, so I copied the struct
// https://rust-lang.github.io/rfcs/2008-non-exhaustive.html#functional-record-updates
#[cw_serde(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    JsonSchema
)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct SupplyResponse {
    /// Always returns a Coin with the requested denom.
    /// This will be of zero amount if the denom does not exist.
    pub amount: Coin,
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<cosmwasm_std::Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_TOKEN_ADDR => self.handle_token_query(msg),
                    MOCK_TOKEN_CONTRACT => self.handle_token_query(msg),
                    MOCK_RATES_CONTRACT => self.handle_rates_query(msg),
                    MOCK_ADDRESS_LIST_CONTRACT => self.handle_addresslist_query(msg),
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            QueryRequest::Bank(bank_query) => match bank_query {
                BankQuery::Supply { denom } => {
                    let response = SupplyResponse {
                        amount: coin(1_000_000, denom),
                    };

                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&response).unwrap()))
                }
                BankQuery::Balance {
                    address: _,
                    denom: _,
                } => {
                    panic!("Unsupported Query")
                }
                BankQuery::AllBalances { address: _ } => {
                    panic!("Unsupported Query")
                }
                BankQuery::DenomMetadata { denom: _ } => {
                    panic!("Unsupported Query")
                }
                BankQuery::AllDenomMetadata { pagination: _ } => {
                    panic!("Unsupported Query")
                }
                _ => panic!("Unsupported Query"),
            },
            _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
        }
    }

    fn handle_token_query(&self, msg: &Binary) -> QuerierResult {
        match from_json(msg).unwrap() {
            Cw721QueryMsg::Tokens { owner, .. } => {
                let res = if owner == MOCK_CONDITIONS_MET_CONTRACT
                    || owner == MOCK_CONDITIONS_NOT_MET_CONTRACT
                {
                    TokensResponse {
                        tokens: MOCK_TOKENS_FOR_SALE
                            [MOCK_TOKENS_FOR_SALE.len() - self.tokens_left_to_burn..]
                            .iter()
                            .copied()
                            .map(String::from)
                            .collect(),
                    }
                } else {
                    TokensResponse {
                        tokens: MOCK_TOKENS_FOR_SALE
                            .iter()
                            .copied()
                            .map(String::from)
                            .collect(),
                    }
                };

                SystemResult::Ok(ContractResult::Ok(to_json_binary(&res).unwrap()))
            }
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
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&res).unwrap()))
            }

            _ => panic!("Unsupported Query"),
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
                                        // Royalty of 10%
                                        amount: coin.amount.multiply_ratio(10u128, 100u128),
                                        denom: coin.denom.clone(),
                                    }],
                                })),
                                // SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                                //     to_address: MOCK_TAX_RECIPIENT.to_owned(),
                                //     amount: vec![Coin {
                                //         // Flat tax of 50
                                //         amount: Uint128::from(50u128),
                                //         denom: coin.denom.clone(),
                                //     }],
                                // })),
                            ],
                        ),
                        Funds::Cw20(_) => {
                            let resp: Response = Response::default();
                            return SystemResult::Ok(ContractResult::Ok(
                                to_json_binary(&resp).unwrap(),
                            ));
                        }
                    };
                    let response = OnFundsTransferResponse {
                        msgs,
                        events: vec![],
                        leftover_funds: new_funds,
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&Some(response)).unwrap()))
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
                _ => SystemResult::Ok(ContractResult::Ok(
                    to_json_binary(&None::<Response>).unwrap(),
                )),
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
