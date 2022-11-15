pub trait Clock {
    fn current_timestamp(&self) -> i64;
}

pub struct FixedClock {
    pub value: i64,
}

impl Clock for FixedClock {
    fn current_timestamp(&self) -> i64 {
        self.value
    }
}
