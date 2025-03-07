use super::mock::{
    add_actors, proper_initialization, query_actors, query_last_mint_timestamp_seconds,
    query_mint_cooldown_minutes, vdf_mint,
};
use andromeda_std::amp::AndrAddr;
use andromeda_std::error::ContractError;
use cosmwasm_std::{testing::mock_env, Addr, BlockInfo, Timestamp, Uint64};

#[test]
fn test_instantiation_private() {
    let (deps, _) = proper_initialization(
        AndrAddr::from_string("cw721"),
        None,
        Some(Uint64::new(5_u64)),
    );
    let actors = query_actors(deps.as_ref()).unwrap().actors;
    assert_eq!(actors, vec![Addr::unchecked("creator")]);

    let res_last_mint_timestamp_seconds =
        query_last_mint_timestamp_seconds(deps.as_ref()).unwrap_err();
    assert_eq!(
        res_last_mint_timestamp_seconds,
        ContractError::CustomError {
            msg: "Not existed".to_string(),
        }
    );

    let mint_cooldown_minutes = query_mint_cooldown_minutes(deps.as_ref())
        .unwrap()
        .mint_cooldown_minutes;
    assert_eq!(mint_cooldown_minutes, Uint64::new(5_u64));
}

#[test]
fn test_add_actors() {
    let (mut deps, _) = proper_initialization(
        AndrAddr::from_string("cw721"),
        None,
        Some(Uint64::new(5_u64)),
    );

    add_actors(
        deps.as_mut(),
        vec![AndrAddr::from_string("actor_1")],
        "creator",
    )
    .unwrap();

    let actors = query_actors(deps.as_ref()).unwrap().actors;
    assert_eq!(
        actors,
        vec![Addr::unchecked("creator"), Addr::unchecked("actor_1")]
    );
}

#[test]
fn test_vdf_mint() {
    let (mut deps, _) = proper_initialization(
        AndrAddr::from_string("cw721"),
        None,
        Some(Uint64::new(5_u64)),
    );

    let mut env = mock_env();
    env.block = BlockInfo {
        height: 100,
        time: Timestamp::from_nanos(1000000000000u64),
        chain_id: "test-chain".to_string(),
    };

    vdf_mint(
        deps.as_mut(),
        "vdf_test_1".to_string(),
        AndrAddr::from_string("nft_recipient"),
        "creator",
        env.clone(),
    )
    .unwrap();

    let last_mint_timestamp_seconds = query_last_mint_timestamp_seconds(deps.as_ref())
        .unwrap()
        .last_mint_timestamp_seconds;
    assert_eq!(last_mint_timestamp_seconds, Uint64::new(1000_u64));

    env.block.time = env.block.time.plus_seconds(200);
    let err = vdf_mint(
        deps.as_mut(),
        "vdf_test_2".to_string(),
        AndrAddr::from_string("nft_recipient"),
        "creator",
        env.clone(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::CustomError {
            msg: "Mint cooldown active".to_string(),
        }
    );

    env.block.time = env.block.time.plus_seconds(100);
    vdf_mint(
        deps.as_mut(),
        "vdf_test_2".to_string(),
        AndrAddr::from_string("nft_recipient"),
        "creator",
        env.clone(),
    )
    .unwrap();

    let last_mint_timestamp_seconds = query_last_mint_timestamp_seconds(deps.as_ref())
        .unwrap()
        .last_mint_timestamp_seconds;
    assert_eq!(last_mint_timestamp_seconds, Uint64::new(1300_u64));
}
