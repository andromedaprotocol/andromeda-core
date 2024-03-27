//use andromeda_ecosystem::anchor_earn::PositionResponse;
use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
use andromeda_std::{
    ado_base::InstantiateMsg, ado_contract::ADOContract, amp::Recipient,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    from_json,
    testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    Coin, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult, Uint128,
    WasmQuery,
};

// This is here since anchor_earn is defunct now.
#[cw_serde]
pub struct PositionResponse {
    pub recipient: Recipient,
    pub aust_amount: Uint128,
}

pub const MOCK_ANCHOR_CONTRACT: &str = "anchor_contract";
pub const MOCK_VAULT_CONTRACT: &str = "vault_contract";

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
                ado_type: "vault".to_string(),
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
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg: _,
            }) => {
                let _ = contract_addr.as_str();
                MockAndromedaQuerier::default().handle_query(&self.base, request)
            }
            _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
        }
    }

    // fn handle_anchor_balance_query(&self, msg: &Binary) -> QuerierResult {
    //     match from_json(msg).unwrap() {
    //         AndromedaQuery::WithdrawableBalance { address } => {
    //             let msg_response = PositionResponse {
    //                 recipient: Recipient::from_string(address),
    //                 aust_amount: Uint128::from(10u128),
    //             };
    //             SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
    //         }
    //         AndromedaQuery::Owner {} => {
    //             let msg_response = ContractOwnerResponse {
    //                 owner: MOCK_VAULT_CONTRACT.to_owned(),
    //             };
    //             SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
    //         }
    //         _ => panic!("Unsupported Query"),
    //     }
    // }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}
