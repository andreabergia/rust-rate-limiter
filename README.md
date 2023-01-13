# Rate-Limit

A sample implementation of a rate limiter in Rust.

## Algorithm

The algorithm selected to compute the request limit is to keep a sliding window of requests, up to a maximum capacity, for each client. In more detail, we associate the timestamp of all the requests we receive to a key that allows distinguishing the client - which could be the source address, a session identifier, or some sort of API key. We keep in memory up to a certain limit of requests, sorted by insertion order. Once we have filled the capacity, we try to see if the oldest requests can be discarded - that is, if they happened enough time ago to be outside the sliding window. If so, we remove them and then add the new request in the window. Otherwise, we deny the request.

## Implementation

We rely on a simple abstraction of a `Clock`, that is able to give us the number of ticks elapsed since the creation, in some unit of measure left.

The algorithm is implemented in `RateLimiter`, which must be created with a clock, the window size in ticks, and the maximum allowed number of requests. The API consists of one method: `RateLimiter::try_add_request`, which returns a `Result` containing whether the request should be allowed, denied, or some information that an error occurred.

The sliding windows are kept in memory in a `HashMap`, associating the requests' keys to a `VecDeque` of the timestamps.
