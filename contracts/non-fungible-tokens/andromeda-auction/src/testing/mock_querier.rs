use andromeda_std::ado_base::InstantiateMsg;
use andromeda_std::ado_contract::ADOContract;

use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
use cosmwasm_schema::cw_serde;

pub use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;
use cosmwasm_std::testing::message_info;
use cosmwasm_std::{coin, BankQuery, Empty, QuerierWrapper};
use cosmwasm_std::{
    from_json,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_json_binary, Binary, Coin, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};

use cw721::msg::{Cw721QueryMsg, OwnerOfResponse, TokensResponse};

pub const MOCK_TOKEN_CONTRACT: &str =
    "cosmwasm1k2mr5h0a6296pe7s7hwttxzvls049wml8zxnpul3apufzu4qwvwsu8c5mn";
pub const MOCK_UNCLAIMED_TOKEN: &str =
    "cosmwasm1h07t2zcl7ce2l9hgkamgsemj00rktgs0ytpnttk7gsfd88awmufqsuwajh";
pub const MOCK_TOKEN_ADDR: &str =
    "cosmwasm1qatal5f83m2ecv6ndxrx6jyj7n2gj6yalvc667eyj705c7pzsatswspwvw";
pub const MOCK_TOKEN_OWNER: &str =
    "cosmwasm1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qlm3aqg";
pub const MOCK_TOKENS_FOR_SALE: &[&str] = &[
    "token1", "token2", "token3", "token4", "token5", "token6", "token7",
];

pub const MOCK_CONDITIONS_MET_CONTRACT: &str = "conditions_met";
pub const MOCK_CONDITIONS_NOT_MET_CONTRACT: &str = "conditions_not_met";

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
    let owner = deps.api.addr_make("owner");
    ADOContract::default()
        .instantiate(
            &mut deps.storage,
            mock_env(),
            &deps.api,
            &QuerierWrapper::new(&deps.querier),
            message_info(&owner, &[]),
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

// NOTE: It's impossible to construct a non_exhaustive struct from another another crate, so I copied the struct
// https://rust-lang.github.io/rfcs/2008-non-exhaustive.html#functional-record-updates
#[cw_serde]
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

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            contract_address: mock_env().contract.address.to_string(),
            tokens_left_to_burn: 2,
        }
    }
}
