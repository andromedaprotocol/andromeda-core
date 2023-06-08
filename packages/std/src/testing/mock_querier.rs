#[cfg(feature = "primitive")]
use crate::ado_base::primitive::{GetValueResponse, Primitive};
use crate::{
    ado_contract::ADOContract,
    amp::{ADO_DB_KEY, VFS_KEY},
    os::adodb::{ActionFee, QueryMsg as ADODBQueryMsg},
    os::kernel::QueryMsg as KernelQueryMsg,
    os::vfs::QueryMsg as VFSQueryMsg,
};
#[cfg(feature = "modules")]
use cosmwasm_std::SubMsg;
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Addr, Binary, Coin, ContractInfoResponse, ContractResult, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
#[cfg(feature = "primitive")]
use cosmwasm_std::{Decimal, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg};
use cw_asset::AssetInfo;

/// Mock CW20 Contract Address
pub const MOCK_CW20_CONTRACT: &str = "cw20_contract";
/// Mock App Contract Address
pub const MOCK_APP_CONTRACT: &str = "app_contract";
/// Mock Primitive Contract Address
pub const MOCK_PRIMITIVE_CONTRACT: &str = "primitive_contract";
/// Mock Kernel Contract Address
pub const MOCK_KERNEL_CONTRACT: &str = "kernel_contract";
/// Mock VFS Contract Address
pub const MOCK_VFS_CONTRACT: &str = "vfs_contract";
/// Mock ADODB Contract Address
pub const MOCK_ADODB_CONTRACT: &str = "adodb_contract";

#[cfg(feature = "modules")]
/// Mock Rates Contract Address
pub const MOCK_RATES_CONTRACT: &str = "rates_contract";
#[cfg(feature = "modules")]
/// Mock Address List Contract Address
pub const MOCK_ADDRESS_LIST_CONTRACT: &str = "address_list_contract";

/// An invalid contract address
pub const INVALID_CONTRACT: &str = "invalid_contract";
/// An invalid VFS Path
pub const FAKE_VFS_PATH: &str = "/f";
/// An invalid ADODB Key
pub const FAKE_ADODB_KEY: &str = "fake_adodb_key";
/// A valid action
pub const MOCK_ACTION: &str = "action";
#[cfg(feature = "modules")]
pub const UNWHITELISTED_ADDRESS: &str = "unwhitelisted_address";
#[cfg(feature = "modules")]
pub const RATES_EXCLUDED_ADDRESS: &str = "rates_excluded_address";

pub struct WasmMockQuerier {
    pub base: MockQuerier,
}

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
        .kernel_address
        .save(
            deps.as_mut().storage,
            &Addr::unchecked(MOCK_KERNEL_CONTRACT),
        )
        .unwrap();
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
    /// A custom query handler that provides custom handling for the mock contract addresses provided in this crate.
    ///
    /// Each contract address has its own handler within the Querier and is called when the contract address is set as such.
    ///
    /// A custom response is added for `cosmwasm_std::ContractInfo` queries that returns a code id of 2 for `INVALID_CONTRACT` and 1 for all other addresses.
    ///
    /// Any other addresses are handled by the default querier.
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
                    #[cfg(feature = "modules")]
                    MOCK_ADDRESS_LIST_CONTRACT => self.handle_address_list_query(msg),
                    _ => panic!("Unsupported query for contract: {}", contract_addr),
                }
            }
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                match contract_addr.as_str() {
                    // MOCK_APP_CONTRACT => self.handle_app_raw_query(key),
                    MOCK_KERNEL_CONTRACT => self.handle_kernel_raw_query(key),
                    MOCK_VFS_CONTRACT => self.handle_kernel_raw_query(key),
                    MOCK_ADODB_CONTRACT => self.handle_adodb_raw_query(key),
                    _ => panic!("Unsupported query for contract: {}", contract_addr),
                }
            }
            // Defaults to code ID 1, returns 2 for `INVALID_CONTRACT` which is considered an invalid ADODB code id
            QueryRequest::Wasm(WasmQuery::ContractInfo { contract_addr }) => {
                let mut resp = ContractInfoResponse::default();
                resp.code_id = match contract_addr.as_str() {
                    INVALID_CONTRACT => 2,
                    _ => 1,
                };
                SystemResult::Ok(ContractResult::Ok(to_binary(&resp).unwrap()))
            }
            _ => self.base.handle_query(request),
        }
    }

    /// Handles all kernel queries.
    ///
    /// Returns the appropriate `MOCK_CONTRACT_*` address for the given key in the case of a `KeyAddress` query.
    ///
    /// Returns `true` for `VerifyAddress` for any address excluding `INVALID_CONTRACT`.
    fn handle_kernel_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            KernelQueryMsg::KeyAddress { key } => match key.as_str() {
                VFS_KEY => {
                    SystemResult::Ok(ContractResult::Ok(to_binary(&MOCK_VFS_CONTRACT).unwrap()))
                }
                ADO_DB_KEY => {
                    SystemResult::Ok(ContractResult::Ok(to_binary(&MOCK_ADODB_CONTRACT).unwrap()))
                }
                &_ => SystemResult::Ok(ContractResult::Err("Invalid Key".to_string())),
            },
            KernelQueryMsg::VerifyAddress { address } => match address.as_str() {
                INVALID_CONTRACT => {
                    SystemResult::Ok(ContractResult::Err("Invalid Address".to_string()))
                }
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&true).unwrap())),
            },
            // _ => SystemResult::Ok(ContractResult::Err("Not implemented".to_string())),
        }
    }

    /// Handles all VFS queries.
    ///
    /// Returns the path provided for `ResolvePath` queries, or an error for`FAKE_PATH`.
    fn handle_vfs_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            VFSQueryMsg::ResolvePath { path } => match path.as_str() {
                FAKE_VFS_PATH => SystemResult::Ok(ContractResult::Err("Invalid Path".to_string())),
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&path).unwrap())),
            },
        }
    }

    /// Handles all App queries.
    ///
    /// Returns `"actual_address"` for `Get` queries.
    fn handle_app_query(&self, _msg: &Binary) -> QuerierResult {
        // match from_binary(msg).unwrap() {
        //     _ => SystemResult::Ok(ContractResult::Err("Error".to_string())),
        // }
        todo!()
    }

    /// Handles all ADODB queries.
    ///
    /// Returns `"ADOType"` for `ADOType` queries with code ID 1 and an error otherwise.
    ///
    /// Returns an error for `CodeId` queries with key `FAKE_ADODB_KEY` and 1 otherwise.
    fn handle_adodb_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            ADODBQueryMsg::ADOType { code_id } => match code_id {
                1 => SystemResult::Ok(ContractResult::Ok(to_binary(&"ADOType").unwrap())),
                _ => SystemResult::Ok(ContractResult::Err("Invalid Code ID".to_string())),
            },
            ADODBQueryMsg::CodeId { ado_type } => match ado_type.as_str() {
                FAKE_ADODB_KEY => SystemResult::Ok(ContractResult::Err("Invalid Key".to_string())),
                _ => SystemResult::Ok(ContractResult::Ok(to_binary(&1).unwrap())),
            },
            _ => SystemResult::Ok(ContractResult::Err("Not implemented".to_string())),
        }
    }

    #[cfg(feature = "primitive")]
    /// Handles all primitive queries.
    ///
    /// Returns a default value for each primitive type.
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

    #[cfg(feature = "modules")]
    /// Handles all address list queries
    ///
    /// Returns `true` for `OnExecute` queries for any address excluding `UNWHITELISTED_ADDRESS`.
    fn handle_address_list_query(&self, msg: &Binary) -> QuerierResult {
        use cosmwasm_std::Response;

        use crate::ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse};
        match from_binary(msg).unwrap() {
            HookMsg::AndrHook(hook) => match hook {
                AndromedaHook::OnExecute { sender, .. } => match sender.as_str() {
                    UNWHITELISTED_ADDRESS => {
                        SystemResult::Ok(ContractResult::Err("Unwhitelisted Address".to_string()))
                    }
                    _ => SystemResult::Ok(ContractResult::Ok(
                        to_binary::<Response>(&Response::default()).unwrap(),
                    )),
                },
                AndromedaHook::OnFundsTransfer { .. } => SystemResult::Ok(ContractResult::Ok(
                    to_binary(&OnFundsTransferResponse::default()).unwrap(),
                )),
                AndromedaHook::OnTokenTransfer { .. } => SystemResult::Ok(ContractResult::Ok(
                    to_binary::<Response>(&Response::default()).unwrap(),
                )),
            },
        }
    }

    #[cfg(feature = "modules")]
    /// Handles all rates queries
    ///
    /// The payments required are calculated using the `calculate_mock_rates_response` method within this crate
    /// unless the sender is assigned as `RATES_EXCLUDED_ADDRESS`.
    fn _handle_rates_query(&self, msg: &Binary) -> QuerierResult {
        use cosmwasm_std::Response;

        use crate::ado_base::hooks::{AndromedaHook, HookMsg, OnFundsTransferResponse};
        match from_binary(msg).unwrap() {
            HookMsg::AndrHook(hook) => match hook {
                AndromedaHook::OnExecute { .. } => SystemResult::Ok(ContractResult::Ok(
                    to_binary::<Response>(&Response::default()).unwrap(),
                )),
                AndromedaHook::OnFundsTransfer { sender, .. } => {
                    if sender.as_str() == RATES_EXCLUDED_ADDRESS {
                        return SystemResult::Ok(ContractResult::Ok(
                            to_binary(&OnFundsTransferResponse::default()).unwrap(),
                        ));
                    }
                    // let msgs = calculate_mock_rates_response(sender, payload, amount);
                    todo!("Implement Rates Query")
                }
                AndromedaHook::OnTokenTransfer { .. } => SystemResult::Ok(ContractResult::Ok(
                    to_binary::<Response>(&Response::default()).unwrap(),
                )),
            },
        }
    }

    /// Handles all CW20 queries.
    ///
    /// Returns a balance of 10 for any `Balance` query.
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

    pub fn handle_kernel_raw_query(&self, key: &Binary) -> QuerierResult {
        let key_vec = key.as_slice();
        let key_str = String::from_utf8(key_vec.to_vec()).unwrap();

        if key_str.contains("kernel_addresses") {
            let split = key_str.split("kernel_addresses");
            let key = split.last();
            if let Some(key) = key {
                match key {
                    VFS_KEY => SystemResult::Ok(ContractResult::Ok(
                        to_binary(&MOCK_VFS_CONTRACT.to_string()).unwrap(),
                    )),
                    ADO_DB_KEY => SystemResult::Ok(ContractResult::Ok(
                        to_binary(&MOCK_ADODB_CONTRACT.to_string()).unwrap(),
                    )),
                    _ => panic!("Invalid Kernel Address Key"),
                }
            } else {
                panic!("Invalid Kernel Address Raw Query")
            }
        } else {
            panic!("Invalid Kernel Raw Query")
        }
    }

    pub fn handle_adodb_raw_query(&self, key: &Binary) -> QuerierResult {
        let key_vec = key.as_slice();
        let key_str = String::from_utf8(key_vec.to_vec()).unwrap();

        if key_str.contains("code_id") {
            let split = key_str.split("code_id");
            let key = split.last();
            if let Some(key) = key {
                match key {
                    FAKE_ADODB_KEY => {
                        SystemResult::Ok(ContractResult::Err("Invalid Key".to_string()))
                    }
                    _ => SystemResult::Ok(ContractResult::Ok(to_binary(&1).unwrap())),
                }
            } else {
                panic!("Invalid ADODB Raw Query")
            }
        } else if key_str.contains("action_fees") {
            let split = key_str.split("action_fees");
            let key = split.last();
            match key {
                Some(key) => {
                    if key.contains("ADOTypeaction") {
                        SystemResult::Ok(ContractResult::Ok(
                            to_binary(&ActionFee::new(
                                MOCK_ACTION.to_string(),
                                AssetInfo::native("uusd"),
                                Uint128::from(10u128),
                            ))
                            .unwrap(),
                        ))
                    } else {
                        SystemResult::Ok(ContractResult::Err("Invalid Key".to_string()))
                    }
                }
                None => SystemResult::Ok(ContractResult::Err("Invalid Key".to_string())),
            }
        } else if key_str.contains("ado_type") {
            let split = key_str.split("ado_type");
            let key = split.last();
            match key {
                Some(key) => match key {
                    "1" => SystemResult::Ok(ContractResult::Ok(to_binary("ADOType").unwrap())),
                    _ => SystemResult::Ok(ContractResult::Err("Invalid Key".to_string())),
                },
                None => SystemResult::Ok(ContractResult::Err("Invalid Key".to_string())),
            }
        } else {
            panic!("Invalid ADODB Raw Query")
        }
    }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}

#[cfg(feature = "modules")]
pub fn calculate_mock_rates_response() -> (Vec<SubMsg>, Vec<Coin>) {
    todo!("Implement after readding rates contract");
}
