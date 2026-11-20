//! Tests for `holder_position::get`.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, Request, StatusCode};
use axum::middleware;
use axum::routing::get as route_get;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use super::holder_position::get;
use super::test_support::{asset, distribution, state_with};
use crate::indexer::Snapshot;
use crate::middleware::field_select;
use crate::models::Holder;
use crate::routes::router;

#[tokio::test]
async fn aggregates_positions_and_claimable() {
    let mut a = asset(1);
    a.total_supply = "1000".to_string();
    a.valuation_usd = 200.0;
    let mut d = distribution(7, 5);
    d.total_amount = "1000".to_string();
    d.distributed = "200".to_string();
    let mut snapshot = Snapshot { assets: vec![a], ..Default::default() };
    snapshot.holders.insert(
        1,
        vec![Holder { address: "GA".into(), balance: "250".into(), share_percent: 25.0 }],
    );
    snapshot.dividends.insert(1, vec![d]);

    let position = get(State(state_with(snapshot)), Path("GA".into())).await.0;
    assert_eq!(position.asset_count, 1);
    assert_eq!(position.total_value_usd, 50.0);
    assert_eq!(position.positions[0].claimable[0].amount, "200");
}

#[tokio::test]
async fn unknown_address_has_empty_position() {
    let position = get(State(state_with(Snapshot::default())), Path("GX".into())).await.0;
    assert_eq!(position.asset_count, 0);
    assert!(position.positions.is_empty());
}

#[tokio::test]
async fn sets_etag_and_honours_if_none_match() {
    let mut snapshot = Snapshot::default();
    snapshot.last_indexed_ledger = 42;

    let app = router(state_with(snapshot));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/holders/GA/position")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let etag = response
        .headers()
        .get(header::ETAG)
        .expect("ETag header should be set")
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(etag, "\"ledger-42\"");

    let cache_control = response
        .headers()
        .get(header::CACHE_CONTROL)
        .expect("Cache-Control header should be set")
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(
        cache_control,
        format!("public, max-age={}", crate::routes::POLL_INTERVAL)
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/holders/GA/position")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(body.is_empty());
}

#[tokio::test]
async fn empty_result_is_well_formed() {
    // Snapshot has holders, but none for the requested address.
    let mut snapshot = Snapshot { assets: vec![asset(1)], ..Default::default() };
    snapshot.holders.insert(
        1,
        vec![Holder { address: "GA".into(), balance: "250".into(), share_percent: 25.0 }],
    );

    let position = get(State(state_with(snapshot)), Path("GX".into())).await.0;
    assert_eq!(position.address, "GX");
    assert_eq!(position.asset_count, 0);
    assert_eq!(position.total_value_usd, 0.0);
    assert!(position.positions.is_empty());

    // Serialized form must be an empty array, not null.
    let json = serde_json::to_value(&position).unwrap();
    assert_eq!(json["address"], "GX");
    assert_eq!(json["asset_count"], 0);
    assert_eq!(json["total_value_usd"], 0.0);
    assert!(json["positions"].is_array());
    assert_eq!(json["positions"].as_array().unwrap().len(), 0);
}

fn router_with(snapshot: Snapshot) -> Router {
    Router::new()
        .route("/holders/:address/position", route_get(get))
        .layer(middleware::from_fn(field_select))
        .with_state(state_with(snapshot))
}

fn populated_snapshot() -> Snapshot {
    let mut a = asset(1);
    a.total_supply = "1000".to_string();
    a.valuation_usd = 200.0;
    let mut snapshot = Snapshot { assets: vec![a], ..Default::default() };
    snapshot.holders.insert(
        1,
        vec![Holder { address: "GA".into(), balance: "250".into(), share_percent: 25.0 }],
    );
    snapshot
}

#[tokio::test]
async fn field_selection_limits_response_keys() {
    let app = router_with(populated_snapshot());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/holders/GA/position?fields=address,asset_count")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert!(obj.contains_key("address"));
    assert!(obj.contains_key("asset_count"));
    assert!(!obj.contains_key("positions"));
    assert!(!obj.contains_key("total_value_usd"));
}

#[tokio::test]
async fn field_selection_unknown_field_errors() {
    let app = router_with(populated_snapshot());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/holders/GA/position?fields=address,nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"], "invalid_parameter");
    let message = json["message"].as_str().unwrap_or_default();
    assert!(message.contains("nope"), "message was: {message}");
}

/// Shared error body shape returned by the API.
#[derive(Debug, serde::Deserialize)]
struct ApiErrorBody {
    error: String,
    message: String,
}

async fn assert_client_error_with_shared_body(
    app: Router,
    uri: &str,
) -> ApiErrorBody {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .expect("router should not panic on malformed input");

    let status = response.status();
    assert!(
        status.is_client_error(),
        "expected a 4xx client error for {uri}, got {status}"
    );
    assert_ne!(status, StatusCode::INTERNAL_SERVER_ERROR);

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body should be readable")
        .to_bytes();

    let body: ApiErrorBody = serde_json::from_slice(&bytes)
        .expect("error response should use the shared {error, message} body shape");
    assert!(!body.error.is_empty(), "error code should be present");
    assert!(!body.message.is_empty(), "error message should be present");
    body
}

#[tokio::test]
async fn malformed_query_parameters_return_client_error() {
    let app = super::holder_position::router(state_with(Snapshot::default()));

    // Non-numeric value for a numeric query parameter.
    assert_client_error_with_shared_body(
        app.clone(),
        "/holder_position/GA?limit=not-a-number",
    )
    .await;

    // Out-of-range value for a bounded query parameter.
    assert_client_error_with_shared_body(
        app.clone(),
        "/holder_position/GA?limit=999999999999999999999999",
    )
    .await;

    // Negative value where a non-negative integer is expected.
    assert_client_error_with_shared_body(
        app.clone(),
        "/holder_position/GA?offset=-1",
    )
    .await;

    // Malformed boolean-ish / enum-ish value.
    assert_client_error_with_shared_body(
        app,
        "/holder_position/GA?include_claimable=maybe",
    )
    .await;
}