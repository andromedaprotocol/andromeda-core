use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, Timestamp};
use cw20::Expiration;

#[cw_serde]
#[derive(Default, Eq, PartialOrd, Copy)]
/// Represents time in milliseconds.
pub struct Milliseconds(pub u64);

impl Milliseconds {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        let time = block.time.seconds() * 1000;
        self.0 <= time
    }

    pub fn is_in_past(&self, block: &BlockInfo) -> bool {
        let time = block.time.seconds() * 1000;
        self.0 < time
    }

    #[inline]
    pub fn zero() -> Milliseconds {
        Milliseconds(0)
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
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
    pub fn milliseconds(&self) -> u64 {
        self.0
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

    pub fn add_milliseconds(&mut self, milliseconds: Milliseconds) {
        self.0 += milliseconds.0;
    }

    pub fn subtract_milliseconds(&mut self, milliseconds: Milliseconds) {
        self.0 -= milliseconds.0;
    }

    pub fn plus_milliseconds(self, milliseconds: Milliseconds) -> Milliseconds {
        Milliseconds(self.0 + milliseconds.0)
    }

    pub fn minus_milliseconds(self, milliseconds: Milliseconds) -> Milliseconds {
        Milliseconds(self.0 - milliseconds.0)
    }

    pub fn add_seconds(&mut self, seconds: u64) {
        self.0 += seconds * 1000;
    }

    pub fn subtract_seconds(&mut self, seconds: u64) {
        if seconds > self.0 / 1000 {
            panic!("Overflow: Cannot subtract seconds from milliseconds")
        }

        self.0 -= seconds * 1000;
    }

    pub fn plus_seconds(self, seconds: u64) -> Milliseconds {
        Milliseconds(self.0 + seconds * 1000)
    }

    pub fn minus_seconds(self, seconds: u64) -> Milliseconds {
        Milliseconds(self.0 - seconds * 1000)
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

impl std::fmt::Display for Milliseconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

            let output = input.is_expired(&env.block);

            assert_eq!(test.is_expired, output, "Test failed: {}", test.name)
        }
    }
}
