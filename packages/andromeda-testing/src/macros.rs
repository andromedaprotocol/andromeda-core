#[macro_export]
macro_rules! ado_deployer {
    ($macro_name:ident, $contract_type:ty, $msg_type:ty) => {
        #[macro_export]
        macro_rules! $macro_name {
            ($contract:expr, $msg_expr:expr, $ado_name:expr) => {{
                let contract: $contract_type = $contract;
                let msg: $msg_type = $msg_expr;
                contract.instantiate(msg, None, &vec![]).unwrap();
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
                &vec![],
            )
            .unwrap()
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
