use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use crate::{
    clock::{Clock, Ticks},
    error::RateLimiterError,
};

#[derive(Debug, Clone)]
struct RequestTimestamp {
    timestamp: i64,
}

impl RequestTimestamp {
    fn new(ticks: Ticks) -> RequestTimestamp {
        RequestTimestamp { timestamp: ticks.0 }
    }
}

#[derive(Debug, Default, Hash, Eq, PartialEq, Clone)]
pub struct SourceAddress(String);

impl SourceAddress {
    pub fn new(address: &str) -> SourceAddress {
        SourceAddress(address.to_string())
    }
}

pub struct RateLimiter<C>
where
    C: Clock,
{
    clock: Arc<Mutex<C>>,
    limit: usize,
    ticks: usize,
    requests: HashMap<SourceAddress, VecDeque<RequestTimestamp>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RequestProcessingResponse {
    Allow,
    Deny,
}

pub type RequestProcessingResult = std::result::Result<RequestProcessingResponse, RateLimiterError>;

impl<C> RateLimiter<C>
where
    C: Clock,
{
    pub fn new(clock: Arc<Mutex<C>>, limit: usize, ticks: usize) -> RateLimiter<C> {
        RateLimiter {
            clock,
            limit,
            ticks,
            requests: HashMap::new(),
        }
    }

    pub fn try_add_request(&mut self, address: SourceAddress) -> RequestProcessingResult {
        let now = RequestTimestamp::new(self.clock.lock()?.ticks_elapsed());
        let requests = self.requests.get(&address);
        if let Some(requests) = requests {
            self.add_to_existing_requests(address, now, requests.clone())
        } else {
            self.add_request_for_new_source(address, now)
        }
    }

    fn add_to_existing_requests(
        &mut self,
        address: SourceAddress,
        now: RequestTimestamp,
        mut requests: VecDeque<RequestTimestamp>,
    ) -> RequestProcessingResult {
        if requests.len() < self.limit {
            requests.push_back(now);
            self.requests.insert(address, requests);
            Ok(RequestProcessingResponse::Allow)
        } else {
            self.check_if_slots_can_be_freed(address, now, requests)
        }
    }

    fn check_if_slots_can_be_freed(
        &mut self,
        address: SourceAddress,
        now: RequestTimestamp,
        mut requests: VecDeque<RequestTimestamp>,
    ) -> RequestProcessingResult {
        while self.can_be_discarded(requests.front(), &now) {
            requests.pop_front();
        }

        if requests.len() < self.limit {
            requests.push_back(now);
            self.requests.insert(address, requests);
            Ok(RequestProcessingResponse::Allow)
        } else {
            Ok(RequestProcessingResponse::Deny)
        }
    }

    fn can_be_discarded(&self, front: Option<&RequestTimestamp>, now: &RequestTimestamp) -> bool {
        match front {
            Some(req) => (req.timestamp + (self.limit * self.ticks) as i64) <= now.timestamp,
            None => false,
        }
    }

    fn add_request_for_new_source(
        &mut self,
        address: SourceAddress,
        now: RequestTimestamp,
    ) -> RequestProcessingResult {
        let requests = VecDeque::from([now]);
        self.requests.insert(address, requests);
        Ok(RequestProcessingResponse::Allow)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        clock::{FixedClock, Ticks},
        rate_limiter::{RateLimiter, RequestProcessingResponse, SourceAddress},
    };

    #[test]
    fn requests_are_independent() {
        let clock = Arc::new(Mutex::new(FixedClock { value: Ticks(100) }));
        let mut rate_limiter = RateLimiter::new(clock, 2, 1);

        let address = SourceAddress::new("1.1.1.1");
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "first request is allowed"
        );
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "second request is allowed"
        );
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Deny,
            "third request is denied"
        );

        let address_2 = SourceAddress::new("2.2.2.2");
        assert_eq!(
            rate_limiter.try_add_request(address_2).unwrap(),
            RequestProcessingResponse::Allow,
            "a request on another address is allowed"
        );
    }

    #[test]
    fn passage_of_time_means_queue_clears_up() {
        let address = SourceAddress::new("1.1.1.1");
        let clock = Arc::new(Mutex::new(FixedClock { value: Ticks(1) }));
        let mut rate_limiter = RateLimiter::new(Arc::clone(&clock), 2, 1);

        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #1 is allowed at time 1"
        );
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #2 is allowed at time 1"
        );
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Deny,
            "request #3 is not allowed at time 1"
        );

        clock.lock().unwrap().value = Ticks(2);
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Deny,
            "request #4 is not allowed at time 2 since slots are used"
        );

        clock.lock().unwrap().value = Ticks(3);
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #5 is allowed at time 3 since time passed and two slots freed"
        );

        clock.lock().unwrap().value = Ticks(4);
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #6 is allowed at time 4 since one slot is free"
        );
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Deny,
            "request #7 is not allowed at time 4 since no slots are free"
        );

        clock.lock().unwrap().value = Ticks(5);
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #7 is allowed at time 5 since one slot is free"
        );
    }

    #[test]
    fn ticks_work() {
        let clock = Arc::new(Mutex::new(FixedClock { value: Ticks(1) }));
        let mut rate_limiter = RateLimiter::new(clock.clone(), 1, 100);

        let address = SourceAddress::new("1.1.1.1");
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #1 is allowed"
        );

        clock.lock().unwrap().value = Ticks(100);
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Deny,
            "request #2 is not allowed at time 100"
        );

        clock.lock().unwrap().value = Ticks(101);
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #3 is again allowed at time 101"
        );
    }
}
