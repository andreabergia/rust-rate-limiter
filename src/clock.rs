use time::OffsetDateTime;

#[derive(Debug, Clone, Copy)]
pub struct Ticks(pub i64);

pub trait Clock {
    fn ticks_elapsed(&self) -> Ticks;
}

pub struct FixedClock {
    pub value: Ticks,
}

impl Clock for FixedClock {
    fn ticks_elapsed(&self) -> Ticks {
        self.value
    }
}

pub struct UnixEpochMillisecondsClock {}

impl Clock for UnixEpochMillisecondsClock {
    fn ticks_elapsed(&self) -> Ticks {
        let nanos = OffsetDateTime::now_utc().unix_timestamp_nanos();
        let millis: i64 = (nanos / 1_000_000)
            .try_into()
            .expect("Should not overflow 64 bits");
        Ticks(millis)
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, UnixEpochMillisecondsClock};

    #[test]
    fn unix_clock_works() {
        let clock = UnixEpochMillisecondsClock {};
        // Approximate timestamp at the time of writing this code
        assert!(clock.ticks_elapsed().0 > 1_669_132_053_000);
    }
}
