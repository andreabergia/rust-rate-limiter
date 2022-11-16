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
