use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Env, Timestamp};
use cw20::Expiration;

#[cw_serde]
/// Represents time in milliseconds.
pub struct Milliseconds(u64);

impl Milliseconds {
    pub fn is_block_expired(&self, env: &Env) -> bool {
        let time = env.block.time.seconds() * 1000;
        self.0 <= time
    }

    #[inline]
    pub fn from_seconds(seconds: u64) -> Milliseconds {
        if seconds > u64::MAX / 1000 {
            panic!("Overflow: Cannot convert seconds to milliseconds")
        }

        Milliseconds(seconds * 1000)
    }

    #[inline]
    pub fn from_nanos(nanos: u64) -> Milliseconds {
        Milliseconds(nanos / 1000000)
    }

    #[inline]
    pub fn seconds(&self) -> u64 {
        self.0 / 1000
    }

    #[inline]
    pub fn nanos(&self) -> u64 {
        if self.0 > u64::MAX / 1000000 {
            panic!("Overflow: Cannot convert milliseconds time to nanoseconds")
        }
        self.0 * 1000000
    }
}

impl From<Milliseconds> for String {
    fn from(time: Milliseconds) -> String {
        time.0.to_string()
    }
}

impl From<Milliseconds> for Timestamp {
    fn from(time: Milliseconds) -> Timestamp {
        Timestamp::from_nanos(time.nanos())
    }
}

impl From<Milliseconds> for Expiration {
    fn from(time: Milliseconds) -> Expiration {
        Expiration::AtTime(time.into())
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::testing::mock_env;

    use super::*;

    struct IsExpiredTestCase {
        name: &'static str,
        input: u64,
        curr_time: u64,
        is_expired: bool,
    }

    #[test]
    fn test_is_expired() {
        let test_cases: Vec<IsExpiredTestCase> = vec![
            IsExpiredTestCase {
                name: "valid expiration time (expired)",
                input: 0,
                curr_time: 1,
                is_expired: true,
            },
            IsExpiredTestCase {
                name: "valid expiration time (not expired)",
                input: 1,
                curr_time: 0,
                is_expired: false,
            },
            IsExpiredTestCase {
                name: "same time (expired)",
                input: 0,
                curr_time: 0,
                is_expired: true,
            },
        ];

        for test in test_cases {
            let input = Milliseconds(test.input);
            let curr_time = Milliseconds(test.curr_time);
            let mut env = mock_env();
            env.block.time = curr_time.into();

            let output = input.is_block_expired(&env);

            assert_eq!(test.is_expired, output, "Test failed: {}", test.name)
        }
    }
}
