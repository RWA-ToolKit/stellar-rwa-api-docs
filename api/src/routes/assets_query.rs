//! `GET /assets` with strict query validation and valuation sorting.
//!
//! Supports `asset_type`, `active`, `sort=valuation`, `order=asc|desc`,
//! `offset` and `limit`. `fields` is handled by [`super::field_select`].

use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::indexer::AppState;
use crate::models::ApiErrorBody;

const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 100;
const KNOWN_PARAMS: [&str; 7] = [
    "asset_type",
    "active",
    "sort",
    "order",
    "offset",
    "limit",
    "fields",
];

pub(crate) fn bad_request(message: String) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiErrorBody {
            error: "invalid_parameter".to_string(),
            message,
        }),
    )
        .into_response()
}

/// Validated list options, or the message describing the first problem.
struct Options {
    asset_type: Option<String>,
    active: Option<bool>,
    sort_valuation: Option<bool>,
    offset: usize,
    limit: usize,
}

fn parse(params: &HashMap<String, String>) -> Result<Options, String> {
    let mut keys: Vec<&String> = params.keys().collect();
    keys.sort();
    if let Some(key) = keys.into_iter().find(|k| !KNOWN_PARAMS.contains(&k.as_str())) {
        return Err(format!(
            "unknown query parameter `{key}`; supported: {}",
            KNOWN_PARAMS.join(", ")
        ));
    }
    let number = |name: &str| -> Result<Option<usize>, String> {
        params
            .get(name)
            .map(|v| {
                v.parse::<usize>()
                    .map_err(|_| format!("`{name}` must be a non-negative integer, got `{v}`"))
            })
            .transpose()
    };
    let active = params
        .get("active")
        .map(|v| {
            v.parse::<bool>()
                .map_err(|_| format!("`active` must be `true` or `false`, got `{v}`"))
        })
        .transpose()?;
    let asset_type = match params.get("asset_type") {
        Some(v) if v.trim().is_empty() => return Err("`asset_type` must not be empty".into()),
        other => other.cloned(),
    };
    let descending = match params.get("order").map(String::as_str) {
        None | Some("desc") => true,
        Some("asc") => false,
        Some(other) => return Err(format!("`order` must be `asc` or `desc`, got `{other}`")),
    };
    let sort_valuation = match params.get("sort").map(String::as_str) {
        None => None,
        Some("valuation") => Some(descending),
        Some(other) => return Err(format!("`sort` must be `valuation`, got `{other}`")),
    };
    if params.contains_key("order") && sort_valuation.is_none() {
        return Err("`order` requires `sort=valuation`".into());
    }
    Ok(Options {
        asset_type,
        active,
        sort_valuation,
        offset: number("offset")?.unwrap_or(0),
        limit: number("limit")?.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
    })
}

/// All tokenized assets, filtered by `asset_type`/`active`, optionally sorted
/// by valuation, then paginated. Invalid parameters answer `400`.
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let options = match parse(&params) {
        Ok(o) => o,
        Err(message) => return bad_request(message),
    };
    // The snapshot is shared behind an `Arc`, so filter by reference and clone
    // only the assets that survive the filters.
    let snapshot = state.snapshot();
    let mut assets = snapshot
        .assets
        .iter()
        .filter(|a| options.asset_type.as_deref().is_none_or(|t| a.asset_type == t))
        .filter(|a| options.active.is_none_or(|active| a.active == active))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(descending) = options.sort_valuation {
        assets.sort_by(|a, b| {
            let ord = a.valuation_usd.total_cmp(&b.valuation_usd).then(a.id.cmp(&b.id));
            if descending {
                ord.reverse()
            } else {
                ord
            }
        });
    } else {
        // When no explicit sort specified, use stable ordering by ID.
        assets.sort_by_key(|a| a.id);
    }
    let page: Vec<_> = assets
        .into_iter()
        .skip(options.offset)
        .take(options.limit)
        .collect();
    Json(page).into_response()
}
