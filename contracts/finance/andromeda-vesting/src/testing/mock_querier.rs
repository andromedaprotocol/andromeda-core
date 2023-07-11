use andromeda_std::ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse};
use andromeda_std::ado_base::InstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::Funds;
use andromeda_std::testing::mock_querier::{
    MockAndromedaQuerier, WasmMockQuerier as AndrMockQuerier,
};
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{coin, BankMsg, CosmosMsg, Response, SubMsg, Uint128};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};

pub use andromeda_std::testing::mock_querier::{
    MOCK_ADDRESS_LIST_CONTRACT, MOCK_APP_CONTRACT, MOCK_KERNEL_CONTRACT, MOCK_RATES_CONTRACT,
};

pub const MOCK_TAX_RECIPIENT: &str = "tax_recipient";
pub const MOCK_ROYALTY_RECIPIENT: &str = "royalty_recipient";

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
                ado_type: "lockdrop".to_string(),
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
                    MOCK_RATES_CONTRACT => self.handle_rates_query(msg),
                    MOCK_ADDRESS_LIST_CONTRACT => self.handle_addresslist_query(msg),
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            //TODO: Need better support for passing the base querier to the custom querier.
            _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
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
                            let resp: Response = Response::default();
                            return SystemResult::Ok(ContractResult::Ok(to_binary(&resp).unwrap()));
                        }
                    };
                    let response = OnFundsTransferResponse {
                        msgs,
                        events: vec![],
                        leftover_funds: new_funds,
                    };
                    SystemResult::Ok(ContractResult::Ok(to_binary(&Some(response)).unwrap()))
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
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&None::<Response>).unwrap())),
            },
        }
    }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}
