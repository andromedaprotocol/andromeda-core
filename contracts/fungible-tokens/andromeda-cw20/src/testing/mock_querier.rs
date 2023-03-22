use andromeda_app::app::QueryMsg as AppQueryMsg;
use andromeda_os::kernel::QueryMsg as KernelQueryMsg;
use andromeda_os::vfs::QueryMsg as VfsQueryMessage;
use common::{
    ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse},
    Funds,
};
use cosmwasm_std::{
    from_binary, from_slice, testing::MockQuerier, to_binary, BankMsg, Binary, Coin,
    ContractResult, CosmosMsg, Querier, QuerierResult, QueryRequest, Response, SubMsg, SystemError,
    SystemResult, Uint128, WasmQuery,
};
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";
pub const MOCK_APP_CONTRACT: &str = "app_contract";
pub const MOCK_ADDRESSLIST_CONTRACT: &str = "addresslist_contract";
pub const MOCK_KERNEL_CONTRACT: &str = "kernel_contract";
pub const MOCK_VFS_CONTRACT: &str = "vfs_contract";
pub const MOCK_TAX_RECIPIENT: &str = "tax_recipient";
pub const MOCK_ROYALTY_RECIPIENT: &str = "royalty_recipient";

pub struct WasmMockQuerier {
    base: MockQuerier,
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
                    MOCK_RATES_CONTRACT => self.handle_rates_query(msg),
                    MOCK_APP_CONTRACT => self.handle_app_query(msg),
                    MOCK_ADDRESSLIST_CONTRACT => self.handle_addresslist_query(msg),
                    MOCK_KERNEL_CONTRACT => self.handle_kernel_query(msg),
                    MOCK_VFS_CONTRACT => self.handle_vfs_query(msg),
                    _ => panic!("Unknown Contract Address {}", contract_addr),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_vfs_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            VfsQueryMessage::ResolvePath { path } => {
                if path.len() < 2 {
                    SystemResult::Ok(ContractResult::Err("InvalidComponent".to_string()))
                } else {
                    SystemResult::Ok(ContractResult::Ok(to_binary(&path).unwrap()))
                }
            } // _ => panic!("Unsupported Query: {:?}", msg),
        }
    }

    fn handle_kernel_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            KernelQueryMsg::KeyAddress { key: _ } => {
                SystemResult::Ok(ContractResult::Ok(to_binary(&MOCK_VFS_CONTRACT).unwrap()))
            }
            _ => panic!("Unsupported Query: {}", msg),
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
}
