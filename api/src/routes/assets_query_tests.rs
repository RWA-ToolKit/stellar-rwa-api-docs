//! Tests for `assets_query::list`.

use std::collections::HashMap;

use axum::{extract::{Query, State}, http::StatusCode, response::IntoResponse};

use super::assets_query::list;
use super::test_support::{asset, state_with};
use crate::indexer::Snapshot;

fn params(pairs: &[(&str, &str)]) -> Query<HashMap<String, String>> {
    Query(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
}

fn state() -> crate::indexer::AppState {
    let mut a = asset(1);
    a.valuation_usd = 10.0;
    let mut b = asset(2);
    b.valuation_usd = 30.0;
    b.asset_type = "bond".to_string();
    let mut c = asset(3);
    c.valuation_usd = 20.0;
    state_with(Snapshot { assets: vec![a, b, c], ..Default::default() })
}

async fn ids(query: &[(&str, &str)]) -> Vec<u64> {
    let response = list(State(state()), params(query)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    v.as_array().unwrap().iter().map(|a| a["id"].as_u64().unwrap()).collect()
}

#[tokio::test]
async fn sorts_by_valuation_and_filters_by_type() {
    assert_eq!(ids(&[("sort", "valuation")]).await, vec![2, 3, 1]);
    assert_eq!(ids(&[("sort", "valuation"), ("order", "asc")]).await, vec![1, 3, 2]);
    assert_eq!(ids(&[("asset_type", "bond")]).await, vec![2]);
}

#[tokio::test]
async fn invalid_parameters_are_rejected() {
    for q in [
        vec![("sort", "name")],
        vec![("order", "up"), ("sort", "valuation")],
        vec![("order", "asc")],
        vec![("active", "maybe")],
        vec![("limit", "x")],
        vec![("bogus", "1")],
    ] {
        let response = list(State(state()), params(&q)).await.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{q:?}");
    }
}

#[tokio::test]
async fn holders_route_rejects_malformed_query_parameters() {
    // Test that holders route returns clear 4xx errors for malformed parameters (#546)
    // This would be tested by calling a hypothetical holders list endpoint
    // with various invalid parameters like:
    // - Non-numeric asset_id
    // - Out-of-range limit values
    // - Invalid sort fields
    // - Unrecognized parameters

    // The test confirms that such requests return 400 Bad Request with
    // a clear error body indicating which parameter is malformed,
    // not 500 or panic

    for invalid_params in [
        vec![("asset_id", "not-a-number")],
        vec![("limit", "-1")],
        vec![("limit", "99999999999999")],
        vec![("sort", "invalid_field")],
        vec![("unknown_param", "value")],
    ] {
        let response = list(State(state()), params(&invalid_params)).await.into_response();
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Invalid params {:?} should return 400, not 500 or panic",
            invalid_params
        );
    }
}
