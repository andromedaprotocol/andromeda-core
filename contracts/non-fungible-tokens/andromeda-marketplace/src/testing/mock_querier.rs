use andromeda_app::app::QueryMsg as AppQueryMsg;
use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
pub use andromeda_std::testing::mock_querier::{
    MOCK_ADDRESS_LIST_CONTRACT, MOCK_APP_CONTRACT, MOCK_KERNEL_CONTRACT,
};
use andromeda_std::{
    ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse},
    ado_contract::ADOContract,
    common::Funds,
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, BankMsg, Binary, Coin, ContractResult, CosmosMsg, OwnedDeps, Querier, QuerierResult,
    QueryRequest, Response, SubMsg, SystemError, SystemResult, Uint128, WasmQuery,
};
use cw721::{Cw721QueryMsg, OwnerOfResponse};

pub const MOCK_ADDRESSLIST_CONTRACT: &str = "addresslist_contract";

pub const MOCK_TOKEN_ADDR: &str = "token0001";
pub const MOCK_TOKEN_OWNER: &str = "owner";
pub const MOCK_UNCLAIMED_TOKEN: &str = "unclaimed_token";
pub const MOCK_TAX_RECIPIENT: &str = "tax_recipient";
pub const MOCK_ROYALTY_RECIPIENT: &str = "royalty_recipient";
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";

pub const RATES: &str = "rates";
use andromeda_std::ado_base::InstantiateMsg;

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
            mock_info("sender", &[]),
            InstantiateMsg {
                ado_type: "crowdfund".to_string(),
                ado_version: "test".to_string(),
                operators: None,
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
                    MOCK_APP_CONTRACT => self.handle_app_query(msg),
                    MOCK_ADDRESSLIST_CONTRACT => self.handle_addresslist_query(msg),
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
        }
    }

    fn handle_app_query(&self, msg: &Binary) -> QuerierResult {
        let valid_identifiers = ["e", "b"];
        match from_binary(msg).unwrap() {
            AppQueryMsg::ComponentExists { name } => {
                let value = valid_identifiers.contains(&name.as_str());
                SystemResult::Ok(ContractResult::Ok(to_binary(&value).unwrap()))
            }
            _ => panic!("Unsupported Query: {}", msg),
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
                                to_binary(&None::<Response>).unwrap(),
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
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&None::<Response>).unwrap())),
            },
        }
    }

    fn handle_addresslist_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            HookMsg::AndrHook(hook_msg) => match hook_msg {
                AndromedaHook::OnExecute { sender, payload: _ } => {
                    let whitelisted_addresses = ["sender"];
                    let response: Response = Response::default();
                    if whitelisted_addresses.contains(&sender.as_str()) {
                        SystemResult::Ok(ContractResult::Ok(to_binary(&response).unwrap()))
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
