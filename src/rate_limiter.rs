use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::{Arc, Mutex, PoisonError},
};

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::clock::Clock;

#[derive(Debug, Clone)]
struct RequestInfo {
    timestamp: i64,
}

impl RequestInfo {
    fn new(timestamp: i64) -> RequestInfo {
        RequestInfo { timestamp }
    }
}

#[derive(Debug, Default, Hash, Eq, PartialEq, Clone)]
pub struct RequestKey {
    address: String,
}

impl RequestKey {
    pub fn new(address: &str) -> RequestKey {
        RequestKey {
            address: address.into(),
        }
    }
}
pub struct RateLimiter<C>
where
    C: Clock,
{
    clock: Arc<Mutex<C>>,
    limit: usize,
    ticks: usize,
    requests: HashMap<RequestKey, VecDeque<RequestInfo>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RequestProcessingResponse {
    Allow,
    Deny,
}

#[derive(Debug)]
pub enum RateLimiterError {
    ThreadingProblem,
}

impl std::error::Error for RateLimiterError {}

impl<C> From<PoisonError<C>> for RateLimiterError {
    fn from(_: PoisonError<C>) -> Self {
        Self::ThreadingProblem
    }
}

impl fmt::Display for RateLimiterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RateLimiterError::ThreadingProblem => write!(f, "Threading problem"),
        }
    }
}

#[derive(Serialize)]
struct Message {
    message: String,
}

impl IntoResponse for RateLimiterError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            RateLimiterError::ThreadingProblem => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(Message {
            message: format!("{}", self),
        });
        (status_code, body).into_response()
    }
}

pub type RequestProcessingResult = std::result::Result<RequestProcessingResponse, RateLimiterError>;
pub type Result<T> = std::result::Result<T, RateLimiterError>;

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

    pub fn try_add_request(&mut self, address: RequestKey) -> RequestProcessingResult {
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
    ) -> RequestProcessingResult {
        let request_info = RequestInfo::new(self.clock.lock()?.current_timestamp());
        if requests.len() < self.limit {
            requests.push_back(request_info);
            self.requests.insert(address, requests);
            Ok(RequestProcessingResponse::Allow)
        } else {
            self.check_if_we_have_free_slot(address, requests, request_info)
        }
    }

    fn add_request_for_new_source(&mut self, address: RequestKey) -> RequestProcessingResult {
        let request_info = RequestInfo::new(self.clock.lock()?.current_timestamp());
        let requests = VecDeque::from([request_info]);
        self.requests.insert(address, requests);
        Ok(RequestProcessingResponse::Allow)
    }

    fn check_if_we_have_free_slot(
        &mut self,
        address: RequestKey,
        mut requests: VecDeque<RequestInfo>,
        request_info: RequestInfo,
    ) -> RequestProcessingResult {
        let now = request_info.timestamp;

        let mut updated = false;
        while self.can_be_discarded(requests.front(), now) {
            requests.pop_front();
            updated = true;
        }

        if requests.len() < self.limit {
            requests.push_back(request_info);
            self.requests.insert(address, requests);
            Ok(RequestProcessingResponse::Allow)
        } else {
            if updated {
                self.requests.insert(address, requests);
            }
            Ok(RequestProcessingResponse::Deny)
        }
    }

    fn can_be_discarded(&self, front: Option<&RequestInfo>, now: i64) -> bool {
        match front {
            Some(req) => (req.timestamp + (self.limit * self.ticks) as i64) <= now,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        clock::FixedClock,
        rate_limiter::{RateLimiter, RequestKey, RequestProcessingResponse},
    };

    #[test]
    fn requests_are_independent() {
        let clock = Arc::new(Mutex::new(FixedClock { value: 100 }));
        let mut rate_limiter = RateLimiter::new(clock, 2, 1);

        let address = RequestKey::new("1.1.1.1");
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

        let address_2 = RequestKey::new("2.2.2.2");
        assert_eq!(
            rate_limiter.try_add_request(address_2).unwrap(),
            RequestProcessingResponse::Allow,
            "a request on another address is allowed"
        );
    }

    #[test]
    fn passage_of_time_means_queue_clears_up() {
        let address = RequestKey::new("1.1.1.1");
        let clock = Arc::new(Mutex::new(FixedClock { value: 1 }));
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

        clock.lock().unwrap().value = 2;
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Deny,
            "request #4 is not allowed at time 2 since slots are used"
        );

        clock.lock().unwrap().value = 3;
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #5 is allowed at time 3 since time passed and two slots freed"
        );

        clock.lock().unwrap().value = 4;
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

        clock.lock().unwrap().value = 5;
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #7 is allowed at time 5 since one slot is free"
        );
    }

    #[test]
    fn ticks_work() {
        let clock = Arc::new(Mutex::new(FixedClock { value: 1 }));
        let mut rate_limiter = RateLimiter::new(clock.clone(), 1, 100);

        let address = RequestKey::new("1.1.1.1");
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #1 is allowed"
        );

        clock.lock().unwrap().value = 100;
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Deny,
            "request #2 is not allowed at time 100"
        );

        clock.lock().unwrap().value = 101;
        assert_eq!(
            rate_limiter.try_add_request(address.clone()).unwrap(),
            RequestProcessingResponse::Allow,
            "request #3 is again allowed at time 101"
        );
    }
}
