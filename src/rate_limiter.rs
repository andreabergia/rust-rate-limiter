use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use crate::clock::Clock;

#[derive(Debug, Clone)]
struct RequestInfo {
    unix_timestamp: i64,
}

impl RequestInfo {
    fn new<C: Clock>(clock: &C) -> RequestInfo {
        RequestInfo {
            unix_timestamp: clock.current_timestamp(),
        }
    }
}

#[derive(Debug, Default, Hash, Eq, PartialEq, Clone)]
struct RequestKey {
    address: String,
}

impl RequestKey {
    fn new(address: &str) -> RequestKey {
        return RequestKey {
            address: address.into(),
        };
    }
}
struct RateLimiter<C>
where
    C: Clock,
{
    clock: Rc<RefCell<C>>,
    limit: usize,
    requests: HashMap<RequestKey, VecDeque<RequestInfo>>,
}

#[derive(Debug, Eq, PartialEq)]
enum RequestProcessingResponse {
    Allow,
    Deny,
}

// type RequestProcessingResult =
//     std::result::Result<RequestProcessingResponse, Box<dyn std::error::Error>>;

impl<C> RateLimiter<C>
where
    C: Clock,
{
    fn new(clock: Rc<RefCell<C>>, limit: usize) -> RateLimiter<C> {
        return RateLimiter {
            clock,
            limit,
            requests: HashMap::new(),
        };
    }

    fn add_request(&mut self, address: RequestKey) -> RequestProcessingResponse {
        let requests = self.requests.get(&address);
        if let Some(requests) = requests {
            self.add_to_existing_requests(address, requests.clone())
        } else {
            self.add_request_for_new_source(address)
        }
    }

    fn add_to_existing_requests(
        &mut self,
        address: RequestKey,
        mut requests: VecDeque<RequestInfo>,
    ) -> RequestProcessingResponse {
        let request_info = RequestInfo::new(&self.clock.borrow() as &C);
        if requests.len() < self.limit {
            requests.push_back(request_info);
            self.requests.insert(address, requests);
            RequestProcessingResponse::Allow
        } else {
            self.check_if_we_have_free_slot(address, requests, request_info)
        }
    }

    fn add_request_for_new_source(&mut self, address: RequestKey) -> RequestProcessingResponse {
        let request_info = RequestInfo::new(&self.clock.borrow() as &C);
        let requests = VecDeque::from([request_info]);
        self.requests.insert(address, requests);
        RequestProcessingResponse::Allow
    }

    fn check_if_we_have_free_slot(
        &mut self,
        address: RequestKey,
        mut requests: VecDeque<RequestInfo>,
        request_info: RequestInfo,
    ) -> RequestProcessingResponse {
        let now = request_info.unix_timestamp;

        let mut updated = false;
        while self.can_be_discarded(requests.front(), now) {
            requests.pop_front();
            updated = true;
        }

        if requests.len() < self.limit {
            requests.push_back(request_info);
            self.requests.insert(address, requests);
            RequestProcessingResponse::Allow
        } else {
            if updated {
                self.requests.insert(address, requests);
            }
            RequestProcessingResponse::Deny
        }
    }

    fn can_be_discarded(&self, front: Option<&RequestInfo>, now: i64) -> bool {
        match front {
            Some(req) => (req.unix_timestamp + self.limit as i64) <= now,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        clock::FixedClock,
        rate_limiter::{RateLimiter, RequestKey, RequestProcessingResponse},
    };

    #[test]
    fn requests_are_independent() {
        let clock = Rc::new(RefCell::new(FixedClock { value: 100 }));
        let mut rate_limiter = RateLimiter::new(clock, 2);

        let address = RequestKey::new("1.1.1.1");
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Allow,
            "first request is allowed"
        );
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Allow,
            "second request is allowed"
        );
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Deny,
            "third request is denied"
        );

        let address_2 = RequestKey::new("2.2.2.2");
        assert_eq!(
            rate_limiter.add_request(address_2),
            RequestProcessingResponse::Allow,
            "a request on another address is allowed"
        );
    }

    #[test]
    fn passage_of_time_means_queue_clears_up() {
        let address = RequestKey::new("1.1.1.1");
        let clock = Rc::new(RefCell::new(FixedClock { value: 1 }));
        let mut rate_limiter = RateLimiter::new(Rc::clone(&clock), 2);

        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Allow,
            "request #1 is allowed at time 1"
        );
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Allow,
            "request #2 is allowed at time 1"
        );
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Deny,
            "request #3 is not allowed at time 1"
        );

        clock.as_ref().borrow_mut().value = 2;
        let r = rate_limiter.add_request(address.clone());
        assert_eq!(
            r,
            RequestProcessingResponse::Deny,
            "request #4 is not allowed at time 2 since slots are used"
        );

        clock.as_ref().borrow_mut().value = 3;
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Allow,
            "request #5 is allowed at time 3 since time passed and two slots freed"
        );

        clock.as_ref().borrow_mut().value = 4;
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Allow,
            "request #6 is allowed at time 4 since one slot is free"
        );
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Deny,
            "request #7 is not allowed at time 4 since no slots are free"
        );

        clock.as_ref().borrow_mut().value = 5;
        assert_eq!(
            rate_limiter.add_request(address.clone()),
            RequestProcessingResponse::Allow,
            "request #7 is allowed at time 5 since one slot is free"
        );
    }
}
