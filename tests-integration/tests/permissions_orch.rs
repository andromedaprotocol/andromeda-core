use andromeda_adodb::ADODBContract;
use andromeda_economics::EconomicsContract;
use andromeda_kernel::KernelContract;
use andromeda_marketplace::MarketplaceContract;
use andromeda_non_fungible_tokens::marketplace::{
    ExecuteMsg, InstantiateMsg as MarketplaceInstantiateMsg, QueryMsg,
};
use andromeda_std::{
    ado_base::{
        permissioning::{PermissionInfo, PermissioningMessage},
        MigrateMsg,
    },
    amp::AndrAddr,
    os::{
        self,
        kernel::{ExecuteMsg as KernelExecuteMsg, InstantiateMsg as KernelInstantiateMsg},
    },
};
use andromeda_vfs::VFSContract;
use cosmwasm_std::Addr;
use cw_orch::prelude::*;
use cw_orch_interchain::prelude::*;

#[test]
fn test_marketplace_migration() {
    // Here `juno-1` is the chain-id and `juno` is the address prefix for this chain
    let sender = Addr::unchecked("sender_for_all_chains");
    let interchain = MockInterchainEnv::new(vec![("juno", &sender.clone().into_string())]);
    let juno = interchain.get_chain("juno").unwrap();

    juno.set_balance(
        sender.clone().into_string().clone(),
        vec![Coin::new(100000000000000, "juno")],
    )
    .unwrap();

    let marketplace_juno = MarketplaceContract::new(juno.clone());
    let kernel_juno = KernelContract::new(juno.clone());
    let vfs_juno = VFSContract::new(juno.clone());
    let adodb_juno = ADODBContract::new(juno.clone());
    let economics_juno = EconomicsContract::new(juno.clone());

    marketplace_juno.upload().unwrap();
    kernel_juno.upload().unwrap();
    vfs_juno.upload().unwrap();
    adodb_juno.upload().unwrap();
    economics_juno.upload().unwrap();

    let kernel_init_msg = &KernelInstantiateMsg {
        owner: None,
        chain_name: "juno".to_string(),
    };
    kernel_juno
        .instantiate(kernel_init_msg, None, None)
        .unwrap();

    vfs_juno
        .instantiate(
            &os::vfs::InstantiateMsg {
                kernel_address: kernel_juno.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    kernel_juno
        .execute(
            &KernelExecuteMsg::UpsertKeyAddress {
                key: "vfs".to_string(),
                value: vfs_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    adodb_juno
        .instantiate(
            &os::adodb::InstantiateMsg {
                kernel_address: kernel_juno.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    kernel_juno
        .execute(
            &KernelExecuteMsg::UpsertKeyAddress {
                key: "adodb".to_string(),
                value: adodb_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    adodb_juno
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: 3,
                ado_type: "rates".to_string(),
                action_fees: None,
                version: "2.0.3".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    let marketplace_init_msg = &MarketplaceInstantiateMsg {
        authorized_cw20_addresses: None,
        authorized_token_addresses: None,
        kernel_address: kernel_juno.address().unwrap().into_string(),
        owner: Some(sender.clone().into_string().clone()),
    };

    marketplace_juno
        .instantiate(marketplace_init_msg, Some(&sender), None)
        .unwrap();

    adodb_juno
        .execute(
            &os::adodb::ExecuteMsg::Publish {
                code_id: 6,
                ado_type: "economics".to_string(),
                action_fees: None,
                version: "1.1.1".to_string(),
                publisher: None,
            },
            None,
        )
        .unwrap();

    economics_juno
        .instantiate(
            &os::economics::InstantiateMsg {
                kernel_address: kernel_juno.address().unwrap().into_string(),
                owner: None,
            },
            None,
            None,
        )
        .unwrap();

    kernel_juno
        .execute(
            &KernelExecuteMsg::UpsertKeyAddress {
                key: "economics".to_string(),
                value: economics_juno.address().unwrap().into_string(),
            },
            None,
        )
        .unwrap();

    marketplace_juno.upload().unwrap();

    // Set up permissions
    marketplace_juno
        .execute(
            &ExecuteMsg::Permissioning(PermissioningMessage::PermissionAction {
                action: "Claim".to_string(),
            }),
            None,
        )
        .unwrap();

    marketplace_juno
        .execute(
            &ExecuteMsg::Permissioning(PermissioningMessage::SetPermission {
                actors: vec![AndrAddr::from_string(sender.clone().into_string())],
                action: "Claim".to_string(),
                permission: andromeda_std::ado_base::permissioning::Permission::Local(
                    andromeda_std::ado_base::permissioning::LocalPermission::Whitelisted {
                        start: None,
                        expiration: None,
                    },
                ),
            }),
            None,
        )
        .unwrap();

    let permissions: Vec<PermissionInfo> = marketplace_juno
        .query(&QueryMsg::Permissions {
            actor: sender.clone().into_string(),
            limit: None,
            start_after: None,
        })
        .unwrap();
    assert_eq!(permissions.len(), 1);

    marketplace_juno.migrate(&MigrateMsg {}, 6).unwrap();

    let permissions: Vec<PermissionInfo> = marketplace_juno
        .query(&QueryMsg::Permissions {
            actor: sender.into_string(),
            limit: None,
            start_after: None,
        })
        .unwrap();
    assert_eq!(permissions.len(), 1);
}
