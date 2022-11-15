use std::collections::HashMap;

#[derive(Debug)]
struct RequestInfo {
    timestamp: i32,
}

impl RequestInfo {
    fn new(timestamp: i32) -> RequestInfo {
        return RequestInfo { timestamp };
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
struct RateLimiter {
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

impl RateLimiter {
    fn new(limit: usize) -> RateLimiter {
        return RateLimiter {
            limit,
            requests: HashMap::new(),
        };
    }

    fn add_request(&mut self, address: &RequestKey, timestamp: i32) -> RequestProcessingResult {
        let requests = self.requests.get_mut(&address);
        if let Some(requests) = requests {
            if requests.len() < self.limit {
                let request_info = RequestInfo::new(timestamp);
                requests.push(request_info);
                Ok(RequestProcessingResponse::Allow)
            } else {
                Ok(RequestProcessingResponse::Deny)
            }
        } else {
            let request_info = RequestInfo::new(timestamp);
            self.requests.insert(address.clone(), vec![request_info]);
            Ok(RequestProcessingResponse::Allow)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{RateLimiter, RequestKey, RequestProcessingResponse};

    #[test]
    fn rate_limiter_works() {
        let mut rate_limiter = RateLimiter::new(2);

        let address = RequestKey::new("1.1.1.1");
        assert_eq!(
            rate_limiter.add_request(&address, 100).unwrap(),
            RequestProcessingResponse::Allow,
            "first request is allowed"
        );
        assert_eq!(
            rate_limiter.add_request(&address, 100).unwrap(),
            RequestProcessingResponse::Allow,
            "second request is allowed"
        );
        assert_eq!(
            rate_limiter.add_request(&address, 100).unwrap(),
            RequestProcessingResponse::Deny,
            "third request is denied"
        );

        let address_2 = RequestKey::new("2.2.2.2");
        assert_eq!(
            rate_limiter.add_request(&address_2, 100).unwrap(),
            RequestProcessingResponse::Allow,
            "a request on another address is allowed"
        );
    }
}

fn main() {
    println!("Hello, world!");
}
