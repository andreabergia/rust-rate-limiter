use time::{Duration, OffsetDateTime};

type UnixTimestamp = i64;

pub trait Clock {
    fn current_timestamp(&self) -> UnixTimestamp;
}

pub struct FixedClock {
    pub value: UnixTimestamp,
}

impl Clock for FixedClock {
    fn current_timestamp(&self) -> UnixTimestamp {
        self.value
    }
}

pub struct MillisecondsUnixClock {}

impl Clock for MillisecondsUnixClock {
    fn current_timestamp(&self) -> UnixTimestamp {
        let nanos = OffsetDateTime::now_utc().unix_timestamp_nanos();
        let millis = nanos / 1_000_000;
        return millis as UnixTimestamp;
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, MillisecondsUnixClock};

    #[test]
    fn unix_clock_works() {
        let clock = MillisecondsUnixClock {};
        // Approximate timestamp at the time of writing this code
        assert!(clock.current_timestamp() > 1_669_132_053_000);
    }
}
