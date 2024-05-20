pub fn get_action_name(contract_name: &str, msg: &str) -> String {
    let suffix = match contract_name.strip_prefix("crates.io:andromeda-") {
        Some(suffix) => {
            let capitalized_suffix: String = suffix
                .split('-')
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().chain(chars).collect(),
                    }
                })
                .collect();
            capitalized_suffix
        }
        None => "".to_string(),
    };

    suffix + msg
}

#[cfg(test)]
mod test {
    use super::*;
    struct TestHandleLocalCase {
        name: &'static str,
        contract_name: &'static str,
        msg: &'static str,
        expected_action_name: String,
    }

    #[test]
    fn test_handle_local() {
        let test_cases = vec![
            TestHandleLocalCase {
                name: "One word contract name, one word ExecuteMsg",
                contract_name: "crates.io:andromeda-crowdfund",
                msg: "Purchase",
                expected_action_name: "CrowdfundPurchase".to_string(),
            },
            TestHandleLocalCase {
                name: "More than one word contract name, more than one word ExecuteMsg",
                contract_name: "crates.io:andromeda-rate-limiting-withdrawals",
                msg: "PurchaseByTokenId",
                expected_action_name: "RateLimitingWithdrawalsPurchaseByTokenId".to_string(),
            },
        ];

        for test in test_cases {
            let res = get_action_name(test.contract_name, test.msg);

            assert_eq!(res, test.expected_action_name, "{}", test.name);
        }
    }
}
