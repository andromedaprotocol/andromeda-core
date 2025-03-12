use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

use andromeda_std::ado_base::InstantiateMsg;
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::testing::mock_querier::MockAndromedaQuerier;
use cosmwasm_std::testing::message_info;
use cosmwasm_std::QuerierWrapper;
use cosmwasm_std::{
    from_json,
    testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    Coin, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};

pub use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;

pub const DEFAULT_VALIDATOR: &str = "default_validator";
pub const VALID_VALIDATOR: &str = "valid_validator";

#[cw_serde]
pub struct Validator {
    /// The operator address of the validator (e.g. cosmosvaloper1...).
    /// See https://github.com/cosmos/cosmos-sdk/blob/v0.47.4/proto/cosmos/staking/v1beta1/staking.proto#L95-L96
    /// for more information.
    ///
    /// This uses `String` instead of `Addr` since the bech32 address prefix is different from
    /// the ones that regular user accounts use.
    pub address: String,
    pub commission: Decimal,
    pub max_commission: Decimal,
    /// The maximum daily increase of the commission
    pub max_change_rate: Decimal,
}

impl Validator {
    /// Creates a new validator.
    ///
    /// If fields get added to the [`Validator`] struct in the future, this constructor will
    /// provide default values for them, but these default values may not be sensible.
    pub fn create(
        address: String,
        commission: Decimal,
        max_commission: Decimal,
        max_change_rate: Decimal,
    ) -> Self {
        Self {
            address,
            commission,
            max_commission,
            max_change_rate,
        }
    }
}

pub fn mock_dependencies_custom(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let default_validator = Validator {
        address: String::from(DEFAULT_VALIDATOR),
        commission: Decimal::percent(1),
        max_commission: Decimal::percent(3),
        max_change_rate: Decimal::percent(1),
    };

    let valid_validator = Validator {
        address: String::from(VALID_VALIDATOR),
        commission: Decimal::percent(1),
        max_commission: Decimal::percent(3),
        max_change_rate: Decimal::percent(1),
    };
    //
    let mut custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));
    //TODO resolve this
    // custom_querier
    //     .base
    //     .staking
    //     .update("uandr", &[default_validator, valid_validator], &[]);
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
            message_info(&Addr::unchecked("sender"), &[]),
            InstantiateMsg {
                ado_type: "splitter".to_string(),
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

    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            contract_address: mock_env().contract.address.to_string(),
            tokens_left_to_burn: 2,
        }
    }
}
