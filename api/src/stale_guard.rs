//! Marks responses as stale when the indexer cannot refresh.

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

use crate::indexer::POLL_INTERVAL;
use crate::poll_status;

/// `true` when the served data is older than the stall threshold or polls are failing.
const STALE: HeaderName = HeaderName::from_static("x-data-stale");
const AGE: HeaderName = HeaderName::from_static("x-data-age-seconds");
const FAILURES: HeaderName = HeaderName::from_static("x-indexer-consecutive-failures");

/// Add `X-Data-Stale: true`, `X-Data-Age-Seconds` and
/// `X-Indexer-Consecutive-Failures` to a response while the indexer is
/// failing or has not polled successfully within the health threshold.
pub async fn stale_headers(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    let failures = poll_status::consecutive_failures();
    let age = poll_status::last_poll_age_seconds();
    let stale = failures > 0 || age.is_some_and(|a| a > poll_status::max_poll_age(POLL_INTERVAL));
    if stale {
        let headers = resp.headers_mut();
        headers.insert(STALE, HeaderValue::from_static("true"));
        if let Some(a) = age {
            headers.insert(AGE, HeaderValue::from(a as u64));
        }
        headers.insert(FAILURES, HeaderValue::from(failures));
    }
    resp
}
