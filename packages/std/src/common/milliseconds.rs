use crate::milliseconds_like;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, Timestamp};
use cw20::Expiration;

#[cw_serde]
#[derive(Default, Eq, PartialOrd, Copy)]
/// Represents time in milliseconds.
pub struct Milliseconds(pub u64);

#[cw_serde]
#[derive(Default, Eq, PartialOrd, Copy)]
pub struct MillisecondsDuration(pub u64);

#[cw_serde]
#[derive(Default, Eq, PartialOrd, Copy)]
pub struct MillisecondsExpiration(pub u64);

milliseconds_like!(MillisecondsExpiration);
milliseconds_like!(MillisecondsDuration);
milliseconds_like!(Milliseconds);

impl From<Milliseconds> for MillisecondsExpiration {
    fn from(value: Milliseconds) -> Self {
        MillisecondsExpiration(value.0)
    }
}

impl From<Milliseconds> for MillisecondsDuration {
    fn from(value: Milliseconds) -> Self {
        MillisecondsDuration(value.0)
    }
}

impl From<MillisecondsDuration> for MillisecondsExpiration {
    fn from(value: MillisecondsDuration) -> Self {
        MillisecondsExpiration(value.0)
    }
}

impl From<MillisecondsDuration> for Milliseconds {
    fn from(value: MillisecondsDuration) -> Self {
        Milliseconds(value.0)
    }
}

impl From<MillisecondsExpiration> for Milliseconds {
    fn from(value: MillisecondsExpiration) -> Self {
        Milliseconds(value.0)
    }
}

#[macro_export]
macro_rules! milliseconds_like {
    ($t: ident) => {
        impl $t {
            pub fn is_expired(&self, block: &BlockInfo) -> bool {
                let time = $t::from_nanos(block.time.nanos());
                self.0 <= time.0
            }

            pub fn is_in_past(&self, block: &BlockInfo) -> bool {
                let time = $t::from_nanos(block.time.nanos());
                self.0 < time.0
            }

            #[inline]
            pub fn zero() -> $t {
                $t(0)
            }

            #[inline]
            pub fn is_zero(&self) -> bool {
                self.0 == 0
            }

            #[inline]
            pub fn from_seconds(seconds: u64) -> $t {
                if seconds > u64::MAX / 1000 {
                    panic!("Overflow: Cannot convert seconds to milliseconds")
                }

                $t(seconds * 1000)
            }

            #[inline]
            pub fn from_nanos(nanos: u64) -> $t {
                $t(nanos / 1000000)
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

            pub fn add_milliseconds(&mut self, milliseconds: $t) {
                self.0 += milliseconds.0;
            }

            pub fn subtract_milliseconds(&mut self, milliseconds: $t) {
                self.0 -= milliseconds.0;
            }

            pub fn plus_milliseconds(self, milliseconds: $t) -> $t {
                $t(self.0 + milliseconds.0)
            }

            pub fn minus_milliseconds(self, milliseconds: $t) -> $t {
                $t(self.0 - milliseconds.0)
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

            pub fn plus_seconds(self, seconds: u64) -> $t {
                $t(self.0 + seconds * 1000)
            }

            pub fn minus_seconds(self, seconds: u64) -> $t {
                $t(self.0 - seconds * 1000)
            }
        }

        impl From<$t> for String {
            fn from(time: $t) -> String {
                time.0.to_string()
            }
        }

        impl From<$t> for Timestamp {
            fn from(time: $t) -> Timestamp {
                Timestamp::from_nanos(time.nanos())
            }
        }

        impl From<$t> for Expiration {
            fn from(time: $t) -> Expiration {
                Expiration::AtTime(time.into())
            }
        }

        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
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
