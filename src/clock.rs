use time::OffsetDateTime;

pub type Ticks = i64;

pub trait Clock {
    fn current_timestamp(&self) -> Ticks;
}

pub struct FixedClock {
    pub value: Ticks,
}

impl Clock for FixedClock {
    fn current_timestamp(&self) -> Ticks {
        self.value
    }
}

pub struct UnixEpochMillisecondsClock {}

impl Clock for UnixEpochMillisecondsClock {
    fn current_timestamp(&self) -> Ticks {
        let nanos = OffsetDateTime::now_utc().unix_timestamp_nanos();
        let millis = nanos / 1_000_000;
        millis as Ticks
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, UnixEpochMillisecondsClock};

    #[test]
    fn unix_clock_works() {
        let clock = UnixEpochMillisecondsClock {};
        // Approximate timestamp at the time of writing this code
        assert!(clock.current_timestamp() > 1_669_132_053_000);
    }
}
