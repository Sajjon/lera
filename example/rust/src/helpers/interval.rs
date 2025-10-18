use crate::prelude::*;

/// A non zero interval in milliseconds
#[derive(Clone, Debug, PartialEq, Eq, Hash, uniffi::Record, derive_more::Deref)]
pub struct Interval {
    ms: u64,
}

impl Interval {
    pub const fn const_try_from(value: u64) -> Result<Self, &'static str> {
        if value == 0 {
            Err("Interval must be non-zero")
        } else {
            Ok(Interval { ms: value })
        }
    }
}

impl From<Interval> for Duration {
    fn from(interval: Interval) -> Self {
        Duration::from_millis(interval.ms)
    }
}
impl TryFrom<u64> for Interval {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::const_try_from(value).map_err(|err| err.to_string())
    }
}
impl Default for Interval {
    fn default() -> Self {
        Self::try_from(1000).unwrap()
    }
}
