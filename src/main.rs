use std::collections::HashMap;

use clock::Clock;

mod clock;

#[derive(Debug)]
struct RequestInfo {
    unix_timestamp: i64,
}

impl RequestInfo {
    fn new(clock: &impl Clock) -> RequestInfo {
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

#[derive(Debug)]
struct RateLimiter<'a, C>
where
    C: Clock,
{
    clock: &'a C,
    limit: usize,
    requests: HashMap<RequestKey, Vec<RequestInfo>>,
}

#[derive(Debug, Eq, PartialEq)]
enum RequestProcessingResponse {
    Allow,
    Deny,
}

type RequestProcessingResult =
    std::result::Result<RequestProcessingResponse, Box<dyn std::error::Error>>;

impl<'a, C> RateLimiter<'a, C>
where
    C: Clock,
{
    fn new(clock: &'a C, limit: usize) -> RateLimiter<C> {
        return RateLimiter {
            clock,
            limit,
            requests: HashMap::new(),
        };
    }

    fn add_request(&mut self, address: &RequestKey) -> RequestProcessingResult {
        let requests = self.requests.get_mut(&address);
        if let Some(requests) = requests {
            if requests.len() < self.limit {
                let request_info = RequestInfo::new(self.clock);
                requests.push(request_info);
                Ok(RequestProcessingResponse::Allow)
            } else {
                Ok(RequestProcessingResponse::Deny)
            }
        } else {
            let request_info = RequestInfo::new(self.clock);
            self.requests.insert(address.clone(), vec![request_info]);
            Ok(RequestProcessingResponse::Allow)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{clock::FixedClock, RateLimiter, RequestKey, RequestProcessingResponse};

    #[test]
    fn rate_limiter_works() {
        let clock = FixedClock { value: 100 };
        let mut rate_limiter = RateLimiter::new(&clock, 2);

        let address = RequestKey::new("1.1.1.1");
        assert_eq!(
            rate_limiter.add_request(&address).unwrap(),
            RequestProcessingResponse::Allow,
            "first request is allowed"
        );
        assert_eq!(
            rate_limiter.add_request(&address).unwrap(),
            RequestProcessingResponse::Allow,
            "second request is allowed"
        );
        assert_eq!(
            rate_limiter.add_request(&address).unwrap(),
            RequestProcessingResponse::Deny,
            "third request is denied"
        );

        let address_2 = RequestKey::new("2.2.2.2");
        assert_eq!(
            rate_limiter.add_request(&address_2).unwrap(),
            RequestProcessingResponse::Allow,
            "a request on another address is allowed"
        );
    }
}

fn main() {
    println!("Hello, world!");
}
