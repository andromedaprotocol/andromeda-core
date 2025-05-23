use andromeda_app::app::QueryMsg as AppQueryMsg;
use andromeda_std::ado_contract::ADOContract;

use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
pub use andromeda_std::testing::mock_querier::{MOCK_APP_CONTRACT, MOCK_KERNEL_CONTRACT};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, from_json,
    testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_json_binary, BankQuery, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult,
    QuerierWrapper, QueryRequest, SystemError, SystemResult, WasmQuery,
};
use cw721::msg::{Cw721QueryMsg, OwnerOfResponse};

pub const MOCK_TOKEN_ADDR: &str =
    "cosmwasm1dkt8wpsymxpna9gktg23henclzgs8vkv4tjcqm0h7qv32kzu0d7sw8kt6u";
pub const MOCK_CW721_ADDR: &str =
    "cosmwasm1jnurcdh67h0xwma5pfps9k9xzrl0gs4yjglkf4z66sc6z6f94frqp5kmk2";
pub const MOCK_TOKEN_OWNER: &str =
    "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";
pub const MOCK_UNCLAIMED_TOKEN: &str =
    "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053x";

pub const _RATES: &str = "rates";
use andromeda_std::ado_base::InstantiateMsg;

pub type TestDeps = cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    WasmMockQuerier,
>;

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
    let sender = deps.api.addr_make("sender");
    ADOContract::default()
        .instantiate(
            &mut deps.storage,
            mock_env(),
            &deps.api,
            &QuerierWrapper::new(&deps.querier),
            message_info(&sender, &[]),
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

#[allow(dead_code)]
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

#[cw_serde]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct OtherSupplyResponse {
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
                    MOCK_APP_CONTRACT => self.handle_app_query(msg),
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            QueryRequest::Bank(bank_query) => match bank_query {
                BankQuery::Supply { denom } => {
                    let response = OtherSupplyResponse {
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
                // BankQuery::DenomMetadata { denom: _ } => {
                //     panic!("Unsupported Query")
                // }
                // BankQuery::AllDenomMetadata { pagination: _ } => {
                //     panic!("Unsupported Query")
                // }
                _ => panic!("Unsupported Query"),
            },
            _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
        }
    }

    fn handle_app_query(&self, msg: &Binary) -> QuerierResult {
        let valid_identifiers = ["e", "b"];
        match from_json(msg).unwrap() {
            AppQueryMsg::ComponentExists { name } => {
                let value = valid_identifiers.contains(&name.as_str());
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&value).unwrap()))
            }
            _ => panic!("Unsupported Query: {msg}"),
        }
    }

    fn handle_token_query(&self, msg: &Binary) -> QuerierResult {
        match from_json::<Cw721QueryMsg>(msg).unwrap() {
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

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            contract_address: mock_env().contract.address.to_string(),
            tokens_left_to_burn: 2,
        }
    }
}
