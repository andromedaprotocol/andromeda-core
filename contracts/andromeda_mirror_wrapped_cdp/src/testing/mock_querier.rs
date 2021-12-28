use andromeda_protocol::mirror_wrapped_cdp::{
    MirrorGovQueryMsg, MirrorLockQueryMsg, MirrorMintQueryMsg, MirrorStakingQueryMsg,
};
use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR},
    to_binary, Binary, Coin, ContractResult, Decimal, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use mirror_protocol::{
    gov::{
        ConfigResponse as GovConfigResponse, PollResponse, PollStatus, PollsResponse,
        SharesResponse, StakerResponse, StateResponse as GovStateResponse, VoteOption,
        VotersResponse, VotersResponseItem,
    },
    lock::{ConfigResponse as LockConfigResponse, PositionLockInfoResponse},
    mint::{
        AssetConfigResponse, ConfigResponse as MintConfigResponse, NextPositionIdxResponse,
        PositionResponse, PositionsResponse,
    },
    staking::{
        ConfigResponse as StakingConfigResponse, PoolInfoResponse, RewardInfoResponse,
        RewardInfoResponseItem,
    },
};
use std::collections::HashMap;
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};
use terraswap::asset::{Asset, AssetInfo};

pub const MOCK_MIRROR_MINT_ADDR: &str = "mirror_mint";
pub const MOCK_MIRROR_STAKING_ADDR: &str = "mirror_staking";
pub const MOCK_MIRROR_GOV_ADDR: &str = "mirror_gov";
pub const MOCK_MIRROR_LOCK_ADDR: &str = "mirror_lock";

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

pub fn mock_lock_config_response() -> LockConfigResponse {
    LockConfigResponse {
        owner: "owner".to_string(),
        mint_contract: MOCK_MIRROR_MINT_ADDR.to_string(),
        base_denom: "base_denom".to_string(),
        lockup_period: 1u64,
    }
}

pub fn mock_asset_config_response() -> AssetConfigResponse {
    AssetConfigResponse {
        token: "token".to_string(),
        auction_discount: Decimal::one(),
        min_collateral_ratio: Decimal::one(),
        end_price: None,
        ipo_params: None,
    }
}

pub fn mock_position_response() -> PositionResponse {
    PositionResponse {
        idx: Uint128::from(1u128),
        owner: "owner".to_string(),
        collateral: Asset {
            amount: Uint128::from(1u128),
            info: AssetInfo::NativeToken {
                denom: "denom".to_string(),
            },
        },
        asset: Asset {
            amount: Uint128::from(1u128),
            info: AssetInfo::NativeToken {
                denom: "denom".to_string(),
            },
        },
        is_short: false,
    }
}
pub fn mock_positions_response() -> PositionsResponse {
    PositionsResponse {
        positions: vec![mock_position_response()],
    }
}

pub fn mock_next_position_idx_response() -> NextPositionIdxResponse {
    NextPositionIdxResponse {
        next_position_idx: Uint128::from(2u128),
    }
}

pub fn mock_pool_info_response() -> PoolInfoResponse {
    PoolInfoResponse {
        asset_token: "asset_token".to_string(),
        staking_token: "staking_token".to_string(),
        total_bond_amount: Uint128::from(1u128),
        total_short_amount: Uint128::from(1u128),
        reward_index: Decimal::one(),
        short_reward_index: Decimal::one(),
        pending_reward: Uint128::from(1u128),
        short_pending_reward: Uint128::from(1u128),
        premium_rate: Decimal::one(),
        short_reward_weight: Decimal::one(),
        premium_updated_time: 1u64,
    }
}

pub fn mock_reward_info_response() -> RewardInfoResponse {
    RewardInfoResponse {
        staker_addr: "staker_addr".to_string(),
        reward_infos: vec![RewardInfoResponseItem {
            asset_token: "asset_token".to_string(),
            bond_amount: Uint128::from(1u128),
            pending_reward: Uint128::from(1u128),
            is_short: false,
        }],
    }
}

pub fn mock_gov_state_response() -> GovStateResponse {
    GovStateResponse {
        poll_count: 1u64,
        total_share: Uint128::from(1u128),
        total_deposit: Uint128::from(1u128),
        pending_voting_rewards: Uint128::from(1u128),
    }
}

pub fn mock_staker_response() -> StakerResponse {
    StakerResponse {
        balance: Uint128::from(1u128),
        share: Uint128::from(1u128),
        locked_balance: vec![],
        withdrawable_polls: vec![],
        pending_voting_rewards: Uint128::from(1u128),
    }
}

pub fn mock_poll_response() -> PollResponse {
    PollResponse {
        id: 1u64,
        creator: "creator".to_string(),
        status: PollStatus::Passed {},
        end_time: 1u64,
        title: "title".to_string(),
        description: "description".to_string(),
        link: None,
        deposit_amount: Uint128::from(1u128),
        execute_data: None,
        yes_votes: Uint128::from(1u128),
        no_votes: Uint128::from(1u128),
        abstain_votes: Uint128::from(1u128),
        total_balance_at_end_poll: None,
        voters_reward: Uint128::from(1u128),
        staked_amount: None,
    }
}

pub fn mock_polls_response() -> PollsResponse {
    PollsResponse {
        polls: vec![mock_poll_response()],
    }
}

pub fn mock_voter_response() -> VotersResponseItem {
    VotersResponseItem {
        voter: "voter".to_string(),
        vote: VoteOption::Yes,
        balance: Uint128::from(1u128),
    }
}

pub fn mock_voters_response() -> VotersResponse {
    VotersResponse {
        voters: vec![mock_voter_response()],
    }
}

pub fn mock_shares_response() -> SharesResponse {
    SharesResponse { stakers: vec![] }
}

pub fn mock_position_lock_info_response() -> PositionLockInfoResponse {
    PositionLockInfoResponse {
        idx: Uint128::from(1u128),
        receiver: "receiver".to_string(),
        locked_amount: Uint128::from(1u128),
        unlock_time: 1u64,
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
    tax_querier: TaxQuerier,
}

#[derive(Clone, Default)]
pub struct TaxQuerier {
    rate: Decimal,
    // this lets us iterate over all pairs that match the first string
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
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
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                if &TerraRoute::Treasury == route {
                    match query_data {
                        TerraQuery::TaxRate {} => {
                            let res = TaxRateResponse {
                                rate: self.tax_querier.rate,
                            };
                            SystemResult::Ok(ContractResult::from(to_binary(&res)))
                        }
                        TerraQuery::TaxCap { denom } => {
                            let cap = self
                                .tax_querier
                                .caps
                                .get(denom)
                                .copied()
                                .unwrap_or_default();
                            let res = TaxCapResponse { cap };
                            SystemResult::Ok(ContractResult::from(to_binary(&res)))
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match contract_addr.as_str() {
                    MOCK_MIRROR_MINT_ADDR => self.handle_mint_query(msg),
                    MOCK_MIRROR_STAKING_ADDR => self.handle_staking_query(msg),
                    MOCK_MIRROR_GOV_ADDR => self.handle_gov_query(msg),
                    MOCK_MIRROR_LOCK_ADDR => self.handle_lock_query(msg),
                    _ => panic!("Unknown contract address"),
                }
            }
            _ => self.base.handle_query(request),
        }
    }

    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            tax_querier: TaxQuerier::default(),
        }
    }

    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    fn handle_mint_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MirrorMintQueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_mint_config_response()).unwrap(),
            )),
            MirrorMintQueryMsg::AssetConfig { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_asset_config_response()).unwrap(),
            )),
            MirrorMintQueryMsg::Position { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_position_response()).unwrap(),
            )),
            MirrorMintQueryMsg::Positions { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_positions_response()).unwrap(),
            )),
            MirrorMintQueryMsg::NextPositionIdx { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_next_position_idx_response()).unwrap(),
            )),
        }
    }

    fn handle_staking_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MirrorStakingQueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_staking_config_response()).unwrap(),
            )),
            MirrorStakingQueryMsg::PoolInfo { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_pool_info_response()).unwrap(),
            )),
            MirrorStakingQueryMsg::RewardInfo { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_reward_info_response()).unwrap(),
            )),
        }
    }

    fn handle_gov_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MirrorGovQueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_gov_config_response()).unwrap(),
            )),
            MirrorGovQueryMsg::State {} => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_gov_state_response()).unwrap(),
            )),
            MirrorGovQueryMsg::Staker { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_staker_response()).unwrap(),
            )),
            MirrorGovQueryMsg::Poll { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_poll_response()).unwrap(),
            )),
            MirrorGovQueryMsg::Polls { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_polls_response()).unwrap(),
            )),
            MirrorGovQueryMsg::Voter { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_voter_response()).unwrap(),
            )),
            MirrorGovQueryMsg::Voters { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_voters_response()).unwrap(),
            )),
            MirrorGovQueryMsg::Shares { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_shares_response()).unwrap(),
            )),
        }
    }

    fn handle_lock_query(&self, msg: &Binary) -> QuerierResult {
        match from_binary(msg).unwrap() {
            MirrorLockQueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_lock_config_response()).unwrap(),
            )),
            MirrorLockQueryMsg::PositionLockInfo { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&mock_position_lock_info_response()).unwrap(),
            )),
        }
    }
}
