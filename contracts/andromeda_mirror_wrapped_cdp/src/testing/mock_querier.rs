use andromeda_protocol::mirror_wrapped_cdp::{
    MirrorGovQueryMsg, MirrorMintQueryMsg, MirrorStakingQueryMsg,
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, Decimal, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use mirror_protocol::{
    gov::ConfigResponse as GovConfigResponse, mint::ConfigResponse as MintConfigResponse,
    staking::ConfigResponse as StakingConfigResponse,
};
use terra_cosmwasm::TerraQueryWrapper;

pub const MOCK_MIRROR_MINT_ADDR: &str = "mirror_mint";
pub const MOCK_MIRROR_STAKING_ADDR: &str = "mirror_staking";
pub const MOCK_MIRROR_GOV_ADDR: &str = "mirror_gov";

pub fn mock_mint_config_response() -> MintConfigResponse {
    MintConfigResponse {
        owner: "owner".to_string(),
        oracle: "oracle".to_string(),
        collector: "collector".to_string(),
        collateral_oracle: "collateral_oracle".to_string(),
        staking: "staking".to_string(),
        terraswap_factory: "terraswap_factory".to_string(),
        lock: "lock".to_string(),
        base_denom: "base_denom".to_string(),
        token_code_id: 1_u64,
        protocol_fee_rate: Decimal::one(),
    }
}

pub fn mock_staking_config_response() -> StakingConfigResponse {
    StakingConfigResponse {
        owner: "owner".to_string(),
        mirror_token: "mirror_token".to_string(),
        terraswap_factory: "terraswap_factory".to_string(),
        base_denom: "base_denom".to_string(),
        mint_contract: "mint_contract".to_string(),
        oracle_contract: "oracle_contract".to_string(),
        premium_min_update_interval: 1_u64,
        short_reward_contract: "short_reward_contract".to_string(),
    }
}

pub fn mock_gov_config_response() -> GovConfigResponse {
    GovConfigResponse {
        owner: "owner".to_string(),
        mirror_token: "mirror_token".to_string(),
        quorum: Decimal::one(),
        threshold: Decimal::one(),
        voting_period: 1_u64,
        effective_delay: 1_u64,
        proposal_deposit: Uint128::from(1_u128),
        voter_weight: Decimal::one(),
        snapshot_period: 1_u64,
    }
}

pub fn mock_dependencies_custom(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_MIRROR_MINT_ADDR => self.handle_mint_query(msg, request),
                    MOCK_MIRROR_STAKING_ADDR => self.handle_staking_query(msg, request),
                    MOCK_MIRROR_GOV_ADDR => self.handle_gov_query(msg, request),
                    _ => panic!("Unknown contract address"),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier { base }
    }

    fn handle_mint_query(
        &self,
        msg: &Binary,
        request: &QueryRequest<TerraQueryWrapper>,
    ) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MirrorMintQueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_mint_config_response()).unwrap(),
            )),
            _ => self.base.handle_query(request),
        }
    }

    fn handle_staking_query(
        &self,
        msg: &Binary,
        request: &QueryRequest<TerraQueryWrapper>,
    ) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MirrorStakingQueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_staking_config_response()).unwrap(),
            )),
            _ => self.base.handle_query(request),
        }
    }

    fn handle_gov_query(
        &self,
        msg: &Binary,
        request: &QueryRequest<TerraQueryWrapper>,
    ) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MirrorGovQueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_gov_config_response()).unwrap(),
            )),
            _ => self.base.handle_query(request),
        }
    }
}
