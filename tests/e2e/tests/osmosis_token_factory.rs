use andromeda_osmosis_token_factory::OsmosisTokenFactoryContract;
use andromeda_socket::osmosis_token_factory::{
    AllLockedResponse, ExecuteMsgFns, FactoryDenomResponse, LockedResponse, QueryMsgFns,
};
use cosmwasm_std::{Addr, Uint128};
use cw_orch::prelude::*;
use cw_orch_daemon::{Daemon, DaemonBase, Wallet};
use std::sync::Mutex;

use e2e::constants::OSMO_5;
use rstest::{fixture, rstest};

// Static mutex to prevent concurrent access to daemon state
static DAEMON_LOCK: Mutex<()> = Mutex::new(());

struct TestCase {
    osmosis_token_factory_contract: OsmosisTokenFactoryContract<DaemonBase<Wallet>>,
    _lock_guard: std::sync::MutexGuard<'static, ()>,
}

const TEST_MNEMONIC: &str = "cereal gossip fox peace youth leader engage move brass sell gas trap issue simple dance source develop black hurt pulp burst predict patient onion";

#[fixture]
fn setup() -> TestCase {
    // Acquire lock to prevent concurrent daemon creation, handle poisoning gracefully
    let lock_guard = DAEMON_LOCK.lock().unwrap_or_else(|poisoned| {
        // Clear the poisoned state and continue
        poisoned.into_inner()
    });

    let daemon = Daemon::builder(OSMO_5)
        .mnemonic(TEST_MNEMONIC)
        .build()
        .unwrap();

    let osmosis_token_factory_contract = OsmosisTokenFactoryContract::new(daemon.clone());

    // Upload and instantiate contract
    osmosis_token_factory_contract.upload().unwrap();
    osmosis_token_factory_contract
        .instantiate(
            &andromeda_socket::osmosis_token_factory::InstantiateMsg {
                kernel_address: "osmo17gxc6ec2cz2h6662tt8wajqaq57kwvdlzl63ceq9keeqm470ywyqrp9qux"
                    .to_string(),
                owner: None,
            },
            None,
            &[],
        )
        .unwrap();
    osmosis_token_factory_contract.set_address(&osmosis_token_factory_contract.address().unwrap());

    TestCase {
        osmosis_token_factory_contract,
        _lock_guard: lock_guard,
    }
}

// Test 1: Create a factory denom directly
#[rstest]
fn test_create_denom(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let contract_addr = osmosis_token_factory_contract.addr_str().unwrap();
    let subdenom = "test_token".to_string();
    let expected_denom = format!("factory/{}/{}", contract_addr, subdenom);

    // Create the denom
    let res = osmosis_token_factory_contract
        .create_denom(subdenom.clone(), &[])
        .unwrap();

    // Verify the response is successful (transaction succeeded)
    assert_eq!(res.code, 0, "Transaction should succeed");

    // Query the token authority to verify the contract is the admin
    let authority_res = osmosis_token_factory_contract
        .token_authority(expected_denom.clone())
        .unwrap();

    // Assert that our contract is the authority for this denom
    assert_eq!(
        authority_res.authority_metadata.unwrap().admin,
        contract_addr
    );

    println!("✅ Created denom: {}", expected_denom);
    println!(
        "✅ Verified contract {} is the denom authority",
        contract_addr
    );
}

// Test 2: Mint tokens for an existing denom
#[rstest]
fn test_mint_tokens(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let contract_addr = osmosis_token_factory_contract.addr_str().unwrap();
    let subdenom = "mint_test".to_string();
    let amount = Uint128::from(1000u128);
    let expected_denom = format!("factory/{}/{}", contract_addr, subdenom);
    // Use None to mint to sender (avoids address validation)
    let recipient = None;

    // First create the denom
    let create_res = osmosis_token_factory_contract
        .create_denom(subdenom.clone(), &[])
        .unwrap();

    // Verify denom creation was successful
    assert_eq!(create_res.code, 0, "Denom creation should succeed");

    // Then mint tokens to sender (None recipient)
    let mint_res = osmosis_token_factory_contract
        .mint(amount, subdenom.clone(), recipient, &[])
        .unwrap();

    // Verify minting was successful
    assert_eq!(mint_res.code, 0, "Token minting should succeed");

    // Verify that our contract is still the authority
    let authority_res = osmosis_token_factory_contract
        .token_authority(expected_denom.clone())
        .unwrap();

    assert_eq!(
        authority_res.authority_metadata.unwrap().admin,
        contract_addr
    );

    println!("✅ Created denom: {}", expected_denom);
    println!("✅ Minted {} tokens of subdenom: {}", amount, subdenom);
    println!(
        "✅ Verified contract {} remains the denom authority",
        contract_addr
    );
}

// Test 3: Query token authority (should be our contract)
#[rstest]
fn test_query_token_authority(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let contract_addr = osmosis_token_factory_contract.addr_str().unwrap();
    let subdenom = "authority_test".to_string();
    let denom = format!("factory/{}/{}", contract_addr, subdenom);

    // Create denom first
    let create_res = osmosis_token_factory_contract
        .create_denom(subdenom.clone(), &[])
        .unwrap();
    assert_eq!(create_res.code, 0, "Denom creation should succeed");

    // Query who has authority over this denom
    let authority_res = osmosis_token_factory_contract
        .token_authority(denom.clone())
        .unwrap();

    // The contract should be the authority
    let authority_metadata = authority_res
        .authority_metadata
        .expect("Authority metadata should exist");
    assert_eq!(authority_metadata.admin, contract_addr);

    // Verify the denom format matches our expectation
    assert_eq!(denom, format!("factory/{}/{}", contract_addr, subdenom));

    println!("✅ Authority for {}: {}", denom, authority_metadata.admin);
    println!("✅ Verified denom format and authority ownership");
}

// Test 4: Query locked amount for CW20 (should be zero initially)
#[rstest]
fn test_query_locked_empty(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let fake_cw20_addr = Addr::unchecked("osmo1fakecw20contractaddress123456789012345678");

    // Query locked amount for non-existent CW20
    let locked_res: LockedResponse = osmosis_token_factory_contract
        .locked(fake_cw20_addr)
        .unwrap();

    println!("Locked amount: {}", locked_res.amount);
    assert_eq!(locked_res.amount, Uint128::zero());
}

// Test 5: Query factory denom for CW20 (should be None initially)
#[rstest]
fn test_query_factory_denom_empty(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let fake_cw20_addr = Addr::unchecked("osmo1fakecw20contractaddress123456789012345678");

    // Query factory denom for non-existent CW20
    let denom_res: FactoryDenomResponse = osmosis_token_factory_contract
        .factory_denom(fake_cw20_addr)
        .unwrap();

    println!("Factory denom: {:?}", denom_res.denom);
    assert_eq!(denom_res.denom, None);
}

// Test 6: Query all locked tokens (should be empty initially)
#[rstest]
fn test_query_all_locked_empty(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    // Query all locked tokens across all CW20s
    let all_locked_res: AllLockedResponse = osmosis_token_factory_contract.all_locked().unwrap();

    println!("All locked tokens: {:?}", all_locked_res.locked);
    assert_eq!(all_locked_res.locked.len(), 0);
}

// Test 7: Test burn functionality (should fail without tokens)
#[rstest]
fn test_burn_without_tokens(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    // Try to burn without sending any tokens - should fail
    let burn_result = osmosis_token_factory_contract.burn(&[]);

    match burn_result {
        Ok(_) => panic!("Expected burn to fail without tokens"),
        Err(e) => {
            println!("✅ Expected error when burning without tokens: {:?}", e);
            // Verify it's the right kind of error (should be related to no funds)
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("funds")
                    || error_msg.contains("amount")
                    || error_msg.contains("coin"),
                "Error should be related to missing funds/tokens"
            );
        }
    }
}

// Test 8: Test unlock functionality (should fail without tokens)
#[rstest]
fn test_unlock_without_tokens(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    // Try to unlock without sending factory tokens - should fail
    let unlock_result = osmosis_token_factory_contract.unlock(None, &[]);

    match unlock_result {
        Ok(_) => panic!("Expected unlock to fail without factory tokens"),
        Err(e) => {
            println!(
                "✅ Expected error when unlocking without factory tokens: {:?}",
                e
            );
            // Verify it's the right kind of error (should be related to no funds)
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("funds")
                    || error_msg.contains("amount")
                    || error_msg.contains("coin")
                    || error_msg.contains("factory"),
                "Error should be related to missing funds/factory tokens"
            );
        }
    }
}

// Test 9: Test minting without creating denom first (should fail)
#[rstest]
fn test_mint_nonexistent_denom(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let nonexistent_subdenom = "does_not_exist".to_string();
    let amount = Uint128::from(100u128);

    // Try to mint tokens for a denom that doesn't exist
    let mint_result = osmosis_token_factory_contract.mint(amount, nonexistent_subdenom, None, &[]);

    match mint_result {
        Ok(_) => panic!("Expected mint to fail for non-existent denom"),
        Err(e) => {
            println!("✅ Expected error when minting non-existent denom: {:?}", e);
            // Verify it's the right kind of error (should be related to denom not existing)
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("denom")
                    || error_msg.contains("not found")
                    || error_msg.contains("exist"),
                "Error should be related to non-existent denom"
            );
        }
    }
}

// Test 10: Simple full workflow - create, mint, burn
#[rstest]
fn test_simple_full_workflow(setup: TestCase) {
    let TestCase {
        osmosis_token_factory_contract,
        ..
    } = setup;

    let contract_addr = osmosis_token_factory_contract.addr_str().unwrap();
    let subdenom = "simple_test".to_string();
    let mint_amount = Uint128::from(1000u128);
    let burn_amount = Uint128::from(300u128);
    let denom = format!("factory/{}/{}", contract_addr, subdenom);

    println!("🚀 Starting simple token factory workflow...");

    // Step 1: Create denom
    println!("📝 Step 1: Creating factory denom...");
    let create_res = osmosis_token_factory_contract
        .create_denom(subdenom.clone(), &[])
        .unwrap();
    assert_eq!(create_res.code, 0, "Denom creation should succeed");
    println!("✅ Created denom: {}", denom);

    // Step 2: Mint tokens
    println!("🪙 Step 2: Minting tokens...");
    let mint_res = osmosis_token_factory_contract
        .mint(mint_amount, subdenom.clone(), None, &[])
        .unwrap();
    assert_eq!(mint_res.code, 0, "Token minting should succeed");
    println!("✅ Minted {} tokens", mint_amount);

    // Step 3: Burn some tokens
    println!("🔥 Step 3: Burning tokens...");
    let burn_coin = Coin {
        denom: denom.clone(),
        amount: burn_amount,
    };
    let burn_res = osmosis_token_factory_contract.burn(&[burn_coin]).unwrap();
    assert_eq!(burn_res.code, 0, "Token burning should succeed");
    println!("✅ Burned {} tokens", burn_amount);

    // Step 4: Verify contract is still authority
    println!("🔍 Step 4: Verifying authority...");
    let authority = osmosis_token_factory_contract
        .token_authority(denom.clone())
        .unwrap();
    assert_eq!(authority.authority_metadata.unwrap().admin, contract_addr);
    println!(
        "✅ Contract {} is still the authority for {}",
        contract_addr, denom
    );

    println!("🎉 Simple workflow completed successfully!");
    println!(
        "📊 Summary: Created → Minted {} → Burned {} → Remaining {}",
        mint_amount,
        burn_amount,
        mint_amount - burn_amount
    );
}
