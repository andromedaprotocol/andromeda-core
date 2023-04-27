#[cfg(feature = "primitive")]
use crate::ado_contract::primitive::{GetValueResponse, Primitive};
use crate::{
    ado_base::{AndromedaQuery, QueryMsg},
    ado_contract::ADOContract,
    amp::ADO_DB_KEY,
    os::adodb::QueryMsg as ADODBQueryMsg,
    os::kernel::QueryMsg as KernelQueryMsg,
    os::vfs::QueryMsg as VFSQueryMsg,
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, Binary, Coin, ContractInfoResponse, ContractResult, Decimal, OwnedDeps,
    Querier, QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse, Cw20QueryMsg};

pub const MOCK_CW20_CONTRACT: &str = "cw20_contract";
pub const MOCK_APP_CONTRACT: &str = "app_contract";
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
pub const MOCK_KERNEL_CONTRACT: &str = "kernel_contract";
pub const MOCK_VFS_CONTRACT: &str = "vfs_contract";
pub const MOCK_ADODB_CONTRACT: &str = "adodb_contract";

pub struct WasmMockQuerier {
    pub base: MockQuerier,
}

pub fn mock_dependencies_custom(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));
    let mut storage = MockStorage::default();
    let mut deps = OwnedDeps {
        storage,
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: std::marker::PhantomData,
    };
    ADOContract::default().kernel_address.save(
        deps.as_mut().storage,
        &Addr::unchecked(MOCK_KERNEL_CONTRACT),
    );
    deps
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
                    MOCK_CW20_CONTRACT => self.handle_cw20_query(msg),
                    MOCK_APP_CONTRACT => self.handle_app_query(msg),
                    #[cfg(feature = "primitive")]
                    MOCK_PRIMITIVE_CONTRACT => self.handle_primitive_query(msg),
                    MOCK_KERNEL_CONTRACT => self.handle_kernel_query(msg),
                    MOCK_VFS_CONTRACT => self.handle_vfs_query(msg),
                    MOCK_ADODB_CONTRACT => self.handle_adodb_query(msg),
                    _ => panic!("Unsupported query for contract: {}", contract_addr),
                }
            }
            // Defaults to code ID 1, returns 2 for `"fake_address"` which is considered an invalid ADODB code id
            QueryRequest::Wasm(WasmQuery::ContractInfo { contract_addr }) => {
                let mut resp = ContractInfoResponse::default();
                resp.code_id = match contract_addr.as_str() {
                    "fake_address" => 2,
                    _ => 1,
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&resp).unwrap()))
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_kernel_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            KernelQueryMsg::KeyAddress { key } => match key.as_str() {
                "vfs" => {
                    SystemResult::Ok(ContractResult::Ok(to_binary(&MOCK_VFS_CONTRACT).unwrap()))
                }
                ADO_DB_KEY => {
                    SystemResult::Ok(ContractResult::Ok(to_binary(&MOCK_ADODB_CONTRACT).unwrap()))
                }
                &_ => SystemResult::Ok(ContractResult::Err("Invalid Key".to_string())),
            },
            KernelQueryMsg::VerifyAddress { address } => match address.as_str() {
                "fake_address" => {
                    SystemResult::Ok(ContractResult::Err("Invalid Address".to_string()))
                }
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&true).unwrap())),
            },
            _ => SystemResult::Ok(ContractResult::Err("Not implemented".to_string())),
        }
    }

    fn handle_vfs_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            VFSQueryMsg::ResolvePath { path } => match path.as_str() {
                "fake_path" => SystemResult::Ok(ContractResult::Err("Invalid Path".to_string())),
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&path).unwrap())),
            },
        }
    }

    fn handle_app_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            QueryMsg::AndrQuery(AndromedaQuery::Get(_)) => {
                SystemResult::Ok(ContractResult::Ok(to_binary(&"actual_address").unwrap()))
            }
            _ => SystemResult::Ok(ContractResult::Err("Error".to_string())),
        }
    }

    fn handle_adodb_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            ADODBQueryMsg::ADOType { code_id } => match code_id {
                1 => SystemResult::Ok(ContractResult::Ok(to_binary(&"ADOType").unwrap())),
                _ => SystemResult::Ok(ContractResult::Err("Invalid Code ID".to_string())),
            },
            ADODBQueryMsg::CodeId { key } => match key.as_str() {
                "fake_key" => SystemResult::Ok(ContractResult::Err("Invalid Key".to_string())),
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&1).unwrap())),
            },
            _ => SystemResult::Ok(ContractResult::Err("Not implemented".to_string())),
        }
    }

    #[cfg(feature = "primitive")]
    fn handle_primitive_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            QueryMsg::AndrQuery(AndromedaQuery::Get(data)) => {
                let res = match data {
                    None => GetValueResponse {
                        key: "default".to_string(),
                        value: Primitive::Decimal(Decimal::zero()),
                    },
                    Some(data) => {
                        let key: String = from_binary(&data).unwrap();
                        match key.as_str() {
                            "String" => GetValueResponse {
                                key,
                                value: Primitive::String("Value".to_string()),
                            },
                            "Uint128" => GetValueResponse {
                                key,
                                value: Primitive::Uint128(Uint128::new(10)),
                            },
                            "Decimal" => GetValueResponse {
                                key,
                                value: Primitive::Decimal(Decimal::percent(1)),
                            },
                            "Coin" => GetValueResponse {
                                key,
                                value: Primitive::Coin(Coin::new(100, "uusd")),
                            },
                            "Bool" => GetValueResponse {
                                key,
                                value: Primitive::Bool(true),
                            },
                            "Vec" => GetValueResponse {
                                key,
                                value: Primitive::Vec(vec![Primitive::from("String".to_string())]),
                            },
                            _ => {
                                return SystemResult::Ok(ContractResult::Err(
                                    "Not Found".to_string(),
                                ))
                            }
                        }
                    }
                };

                SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
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

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}
