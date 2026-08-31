//! `GET /events`.

use axum::{extract::State, Json};

use crate::indexer::AppState;
use crate::models::Event;

/// Recent contract events captured by the indexer.
pub async fn list(State(state): State<AppState>) -> Json<Vec<Event>> {
    Json(state.snapshot().events)
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
        Router::new().route("/events", get(super::list)).with_state(AppState::for_test(
            crate::indexer::Config {
                rpc_url: "https://soroban-testnet.stellar.org".to_string(),
                registry_id: "CBX5SMLTXX6JP4HA5GQIO2V6QM7WCUGL2GZ6D4U773HMRI6RXISKPUR3".to_string(),
                dividend_id: "CAR4XY3CEBQWFOL27JEWFW34KXSIZA7RFKDQMEIV7ZU723RWY37I2SYX".to_string(),
                read_source: "GAIQGTOBTTLLDJ4SWGGESM7UWJ2DI4K3ZNHUSHPDKJL2IE5FKY3BSRAA".to_string(),
            },
            metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder().handle(),
            Snapshot { events, ..Snapshot::default() },
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
            .oneshot(Request::builder().uri("/events").body(Body::empty()).unwrap())
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
}
