//! `GET /v1/events`.
//!
//! Returns the most recent contract events ingested by the indexer, newest
//! first. The indexer polls Soroban RPC `getEvents` every 10 seconds and
//! keeps a bounded in-memory ring buffer of up to 200 events.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::indexer::AppState;
use crate::models::Event;

/// Maximum events returned per request.
const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

#[derive(Deserialize)]
pub struct EventsQuery {
    /// Maximum number of events to return (default 50, max 200).
    limit: Option<usize>,
}

/// Recent contract events captured by the indexer.
///
/// Events are ordered newest-first. The indexer retains up to 200 events in
/// memory; the buffer rolls over as new events arrive. A failed RPC cycle
/// carries the previous event list forward unchanged rather than returning an
/// empty array.
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<EventsQuery>,
) -> Json<Vec<Event>> {
    let limit = params
        .limit
        .unwrap_or(DEFAULT_LIMIT)
        .min(MAX_LIMIT);
    let events = state.snapshot().events.clone();
    Json(events.into_iter().take(limit).collect())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt as _;

    use crate::indexer::{AppState, Snapshot};
    use crate::models::Event;

    fn app(events: Vec<Event>) -> Router {
        Router::new()
            .route("/events", get(super::list))
            .with_state(AppState::for_test(
                crate::indexer::Config {
                    rpc_url: "https://soroban-testnet.stellar.org".to_string(),
                    registry_id: "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3"
                        .to_string(),
                    dividend_id: "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX"
                        .to_string(),
                    read_source: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA"
                        .to_string(),
                },
                metrics_exporter_prometheus::PrometheusBuilder::new()
                    .build_recorder()
                    .handle(),
                Snapshot {
                    events,
                    ..Snapshot::default()
                },
            ))
    }

    #[tokio::test]
    async fn list_events_returns_recent_events() {
        let events = vec![Event {
            id: 1,
            contract: "CA...".to_string(),
            event_type: "Transfer".to_string(),
            ledger: 42,
            timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            data: serde_json::json!({"from": "A", "to": "B", "amount": "10"}),
        }];

        let response = app(events)
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert!(value.is_array());
        assert_eq!(value[0]["event_type"], "Transfer");
        assert_eq!(value[0]["ledger"], 42);
    }

    #[tokio::test]
    async fn list_events_respects_limit() {
        let events: Vec<Event> = (1..=10)
            .map(|i| Event {
                id: i,
                contract: format!("C{i}"),
                event_type: "Transfer".to_string(),
                ledger: i as u32,
                timestamp: None,
                data: serde_json::Value::Null,
            })
            .collect();

        let response = app(events)
            .oneshot(
                Request::builder()
                    .uri("/events?limit=3")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value.as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn list_events_empty_returns_empty_array() {
        let response = app(vec![])
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value, serde_json::json!([]));
    }
}
