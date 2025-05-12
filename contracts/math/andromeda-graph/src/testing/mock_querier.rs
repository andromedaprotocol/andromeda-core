use andromeda_math::point::{GetDataOwnerResponse, PointCoordinate, QueryMsg as PointQueryMsg};
use andromeda_std::amp::AndrAddr;
use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
use andromeda_std::{
    ado_base::InstantiateMsg, ado_contract::ADOContract,
    testing::mock_querier::MOCK_KERNEL_CONTRACT,
};
use cosmwasm_std::{
    from_json,
    testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    Coin, ContractInfoResponse, OwnedDeps, Querier, QuerierResult, QueryRequest, SignedDecimal,
    SystemError, SystemResult, WasmQuery,
};
use cosmwasm_std::{to_json_binary, Binary, ContractResult};
use cosmwasm_std::{Addr, QuerierWrapper};

pub const SENDER: &str = "cosmwasm1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qlm3aqg";
pub const CREATOR: &str = "cosmwasm1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqs8s7vcp";
pub const MOCK_POINT_CONTRACT: &str =
    "cosmwasm15rgnutlhayzvrl73q3xgnxlt93dnr2ma8gcgy2md05jp6tn0aszssq7036";

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
            message_info(&Addr::unchecked(SENDER), &[]),
            InstantiateMsg {
                ado_type: "graph".to_string(),
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
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_POINT_CONTRACT => self.handle_point_smart_query(msg),
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            QueryRequest::Wasm(WasmQuery::ContractInfo { contract_addr }) => {
                match contract_addr.as_str() {
                    MOCK_POINT_CONTRACT => self.handle_point_contract_info_query(),
                    _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
                }
            }
            _ => MockAndromedaQuerier::default().handle_query(&self.base, request),
        }
    }

    fn handle_point_smart_query(&self, msg: &Binary) -> QuerierResult {
        match from_json(msg).unwrap() {
            PointQueryMsg::GetPoint {} => {
                let msg_response = PointCoordinate {
                    x_coordinate: SignedDecimal::from_ratio(10, 1),
                    y_coordinate: SignedDecimal::from_ratio(10, 1),
                    z_coordinate: Some(SignedDecimal::from_ratio(10, 1)),
                };
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
            }
            PointQueryMsg::GetDataOwner {} => {
                let msg_response = GetDataOwnerResponse {
                    owner: AndrAddr::from_string(SENDER.to_string()),
                };
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
            }
            _ => panic!("Unsupported Query"),
        }
    }

    fn handle_point_contract_info_query(&self) -> QuerierResult {
        let msg_response =
            ContractInfoResponse::new(5, Addr::unchecked(CREATOR), None, false, None);
        SystemResult::Ok(ContractResult::Ok(to_json_binary(&msg_response).unwrap()))
    }

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }
}
