//! `?fields=a,b` field selection for every JSON data route.
//!
//! Implemented as middleware so all list endpoints (assets, holders, events,
//! dividends, ...) behave the same. The `fields` parameter is stripped from
//! the request, the handler runs normally, and a successful JSON object or
//! array response is projected onto the requested fields. A field that does
//! not exist on the records answers `400`.

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode, Uri},
    middleware::Next,
    response::Response,
};
use serde_json::{Map, Value};

use super::assets_query::bad_request;

/// Split `fields` out of the query string: returns (remaining query, fields).
fn take_fields(query: &str) -> (String, Option<String>) {
    let mut fields = None;
    let mut rest = Vec::new();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        match pair.strip_prefix("fields=") {
            Some(value) => fields = Some(value.replace("%2C", ",").replace("%2c", ",")),
            None => rest.push(pair),
        }
    }
    (rest.join("&"), fields)
}

fn project(object: &Map<String, Value>, fields: &[String]) -> Value {
    Value::Object(
        fields
            .iter()
            .filter_map(|f| object.get(f).map(|v| (f.clone(), v.clone())))
            .collect(),
    )
}

pub async fn field_select(mut req: Request<Body>, next: Next) -> Response {
    let (remaining, raw) = match req.uri().query() {
        Some(q) => take_fields(q),
        None => return next.run(req).await,
    };
    let Some(raw) = raw else {
        return next.run(req).await;
    };
    let fields: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(str::to_owned)
        .collect();

    let mut parts = req.uri().clone().into_parts();
    let path = req.uri().path();
    let pq = if remaining.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{remaining}")
    };
    if let Ok(pq) = pq.parse() {
        parts.path_and_query = Some(pq);
        if let Ok(uri) = Uri::from_parts(parts) {
            *req.uri_mut() = uri;
        }
    }

    let response = next.run(req).await;
    if fields.is_empty() || response.status() != StatusCode::OK {
        return response;
    }
    let (mut head, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, usize::MAX).await else {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .expect("static response is well-formed");
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return Response::from_parts(head, Body::from(bytes));
    };

    let sample = match &value {
        Value::Array(items) => items.first().and_then(Value::as_object),
        Value::Object(object) => Some(object),
        _ => None,
    };
    let Some(sample) = sample else {
        return Response::from_parts(head, Body::from(bytes));
    };
    let mut known: Vec<&String> = sample.keys().collect();
    known.sort();
    if let Some(unknown) = fields.iter().find(|f| !sample.contains_key(*f)) {
        let list = known.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", ");
        return bad_request(format!("unknown field `{unknown}`; available fields: {list}"));
    }

    let projected = match &value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .filter_map(Value::as_object)
                .map(|o| project(o, &fields))
                .collect(),
        ),
        Value::Object(object) => project(object, &fields),
        other => other.clone(),
    };
    let body = serde_json::to_vec(&projected).expect("projection serializes");
    head.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(head, Body::from(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use serde_json::json;
    use tower::ServiceExt;

    /// Minimal stand-in for the assets route response shape: a JSON array of
    /// asset records. Exercises the same projection path the real route uses.
    async fn assets() -> Response {
        let body = json!([
            {"asset_id": "a1", "name": "Alpha", "supply": 100},
            {"asset_id": "a2", "name": "Beta", "supply": 200},
        ]);
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&body).expect("serializes")))
            .expect("static response is well-formed")
    }

    fn app() -> Router {
        Router::new()
            .route("/assets", get(assets))
            .layer(axum::middleware::from_fn(field_select))
    }

    async fn body_json(response: Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body collects");
        serde_json::from_slice(&bytes).expect("body is json")
    }

    #[tokio::test]
    async fn assets_route_projects_selected_fields() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/assets?fields=asset_id,name")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
        let value = body_json(response).await;
        assert_eq!(
            value,
            json!([
                {"asset_id": "a1", "name": "Alpha"},
                {"asset_id": "a2", "name": "Beta"},
            ])
        );
    }

    #[tokio::test]
    async fn assets_route_unknown_field_is_bad_request() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/assets?fields=asset_id,nope")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let value = body_json(response).await;
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .expect("error body has message");
        assert!(message.contains("unknown field `nope`"), "got: {message}");
        assert!(message.contains("asset_id"), "got: {message}");
    }
}
