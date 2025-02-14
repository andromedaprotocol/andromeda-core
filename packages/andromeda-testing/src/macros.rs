#[macro_export]
macro_rules! ado_deployer {
    ($macro_name:ident, $contract_type:ty, $msg_type:ty) => {
        #[macro_export]
        macro_rules! $macro_name {
            ($contract:expr, $msg_expr:expr, $ado_name:expr) => {{
                let contract: $contract_type = $contract;
                let msg: $msg_type = $msg_expr;
                contract.instantiate(msg, None, None).unwrap();
                contract
            }};
        }
    };
}

#[macro_export]
macro_rules! register_ado {
    ($chain:expr, $contract:expr, $ado_type:expr) => {{
        ($chain)
            .aos
            .adodb
            .execute(
                &os::adodb::ExecuteMsg::Publish {
                    code_id: $contract.code_id().unwrap(),
                    ado_type: $ado_type.to_string(),
                    version: "1.0.0".to_string(),
                    publisher: None,
                    action_fees: None,
                },
                None,
            )
            .unwrap()
    }};
}

#[macro_export]
macro_rules! execute_ibc_transfer {
    ($chain2:expr, $message:expr, $denom:expr, $amount:expr) => {{
        let tx = $chain2
            .aos
            .kernel
            .execute(
                &os::kernel::ExecuteMsg::Send { message: $message },
                Some(&[Coin {
                    denom: $denom,
                    amount: $amount,
                }]),
            )
            .unwrap();
        tx
    }};
}

#[macro_export]
macro_rules! verify_balances {
    ($chain:expr, $addresses:expr, $ibc_denom:expr) => {{
        let balance1 = $chain
            .chain
            .query_all_balances($addresses[0].clone())
            .unwrap();
        let balance2 = $chain
            .chain
            .query_all_balances($addresses[1].clone())
            .unwrap();

        assert_eq!(balance1[0].denom, $ibc_denom);
        assert_eq!(balance2[0].denom, $ibc_denom);
        assert_eq!(balance1[0].amount, Uint128::new(60));
        assert_eq!(balance2[0].amount, Uint128::new(40));
    }};
}

#[macro_export]
macro_rules! create_ibc_message {
    ($target_chain:expr, $contract_addr:expr, $exec_msg:expr, $funds:expr) => {{
        AMPMsg::new(
            AndrAddr::from_string(format!(
                "ibc://{}/{}",
                $target_chain.chain_name, $contract_addr
            )),
            to_json_binary($exec_msg).unwrap(),
            $funds,
        )
    }};
}
