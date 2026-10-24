//! Tests for the assets route covering malformed and out-of-range query
//! parameters. These ensure the route responds with a clear 4xx (using the
//! shared error body shape) rather than a 500 or a panic.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::routes::assets::router;
use crate::routes::error_body::ErrorBody;

async fn get_assets(uri: &str) -> (StatusCode, Option<ErrorBody>) {
    let app = router();
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("failed to build request"),
        )
        .await
        .expect("router failed to respond");

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");

    let error = if body.is_empty() {
        None
    } else {
        serde_json::from_slice::<ErrorBody>(&body).ok()
    };

    (status, error)
}

fn assert_client_error(status: StatusCode) {
    assert!(
        status.is_client_error(),
        "expected a 4xx client error, got {status}"
    );
}

#[tokio::test]
async fn assets_rejects_non_numeric_limit() {
    let (status, error) = get_assets("/assets?limit=not-a-number").await;
    assert_client_error(status);
    let error = error.expect("expected shared error body shape");
    assert!(!error.message.is_empty(), "error message should be present");
}

#[tokio::test]
async fn assets_rejects_out_of_range_limit() {
    let (status, error) = get_assets("/assets?limit=999999999999999999999").await;
    assert_client_error(status);
    let error = error.expect("expected shared error body shape");
    assert!(!error.message.is_empty(), "error message should be present");
}

#[tokio::test]
async fn assets_rejects_non_numeric_offset() {
    let (status, error) = get_assets("/assets?offset=abc").await;
    assert_client_error(status);
    let error = error.expect("expected shared error body shape");
    assert!(!error.message.is_empty(), "error message should be present");
}

#[tokio::test]
async fn assets_rejects_negative_offset() {
    let (status, error) = get_assets("/assets?offset=-1").await;
    assert_client_error(status);
    let error = error.expect("expected shared error body shape");
    assert!(!error.message.is_empty(), "error message should be present");
}

#[tokio::test]
async fn assets_rejects_malformed_field_selection() {
    let (status, error) = get_assets("/assets?fields=,,,").await;
    assert_client_error(status);
    let error = error.expect("expected shared error body shape");
    assert!(!error.message.is_empty(), "error message should be present");
}
