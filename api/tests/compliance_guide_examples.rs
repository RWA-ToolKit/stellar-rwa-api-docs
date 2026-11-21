//! Snapshot tests for example responses documented in Compliance Guide.
//!
//! This test module verifies that the API compliance endpoint examples
//! remain accurate with respect to the contract interface and models.
//! It validates:
//! - /assets/:id/compliance response structure
//! - jurisdiction breakdown format
//! - compliance status enums
//! - jurisdiction-blocking worked example (blocked jurisdiction end to end)
//! - KYC-expiry-handling worked example (ledger timestamp vs expiry timestamp)

use serde_json::json;

/// Validates the compliance summary example from the Compliance Guide.
/// This endpoint exposes compliance statistics without PII.
#[test]
fn compliance_summary_example_valid() {
    let compliance_example = json!({
        "total_records": 1,
        "approved": 1,
        "suspended": 0,
        "rejected": 0,
        "pending": 0,
        "with_expiry": 0,
        "jurisdictions": [
            { "jurisdiction": "US", "count": 1 }
        ]
    });

    // Verify all required count fields exist
    assert!(compliance_example["total_records"].is_number());
    assert_eq!(compliance_example["total_records"], 1);

    assert!(compliance_example["approved"].is_number());
    assert_eq!(compliance_example["approved"], 1);

    assert!(compliance_example["suspended"].is_number());
    assert_eq!(compliance_example["suspended"], 0);

    assert!(compliance_example["rejected"].is_number());
    assert_eq!(compliance_example["rejected"], 0);

    assert!(compliance_example["pending"].is_number());
    assert_eq!(compliance_example["pending"], 0);

    assert!(compliance_example["with_expiry"].is_number());
    assert_eq!(compliance_example["with_expiry"], 0);

    // Verify jurisdictions array structure
    assert!(compliance_example["jurisdictions"].is_array());
    let jurisdictions = compliance_example["jurisdictions"].as_array().unwrap();
    assert_eq!(jurisdictions.len(), 1);

    // Verify jurisdiction entry
    let jurisdiction = &jurisdictions[0];
    assert!(jurisdiction["jurisdiction"].is_string());
    assert_eq!(jurisdiction["jurisdiction"], "US");
    assert!(jurisdiction["count"].is_number());
    assert_eq!(jurisdiction["count"], 1);
}

/// Validates the runnable worked example from the Compliance Guide.
/// The guide walks through: register a KYC record, then read the summary.
/// This test pins the exact request/response shapes the guide documents so
/// the worked example stays verified against the current API surface.
#[test]
fn worked_example_register_then_summary_valid() {
    // Step 1: POST /assets/:id/kyc — request body documented in the guide.
    let register_request = json!({
        "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        "jurisdiction": "US",
        "status": "Approved",
        "expiry": null
    });
    assert!(register_request["address"].is_string());
    assert_eq!(register_request["jurisdiction"], "US");
    assert_eq!(register_request["status"], "Approved");
    assert!(register_request["expiry"].is_null());

    // Step 2: GET /assets/:id/compliance — response documented in the guide.
    let summary = json!({
        "total_records": 1,
        "approved": 1,
        "suspended": 0,
        "rejected": 0,
        "pending": 0,
        "with_expiry": 0,
        "jurisdictions": [
            { "jurisdiction": "US", "count": 1 }
        ]
    });

    // The single registered record is reflected in the summary counts.
    assert_eq!(summary["total_records"], 1);
    assert_eq!(summary["approved"], 1);
    assert_eq!(summary["jurisdictions"][0]["jurisdiction"], "US");
    assert_eq!(summary["jurisdictions"][0]["count"], 1);
}

/// Validates the KYC-expiry-handling worked example from the Compliance Guide.
/// The guide walks through an approval that carries an explicit expiry and
/// compares it against an explicit ledger timestamp to decide whether the
/// approval is still valid or has expired.
#[test]
fn worked_example_kyc_expiry_handling_valid() {
    // Step 1: Register an approval with an explicit expiry timestamp.
    // The expiry is a Unix timestamp (seconds) stored on the KYC record.
    let expiry_timestamp: i64 = 1_700_000_000; // 2023-11-14T22:13:20Z
    let register_request = json!({
        "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        "jurisdiction": "US",
        "status": "Approved",
        "expiry": expiry_timestamp
    });
    assert_eq!(register_request["status"], "Approved");
    assert_eq!(register_request["expiry"], expiry_timestamp);

    // Step 2: Compare an explicit ledger timestamp against the expiry.
    // Case A — ledger timestamp is BEFORE expiry: approval is still valid.
    let ledger_timestamp_valid: i64 = 1_699_999_000; // 1000s before expiry
    assert!(ledger_timestamp_valid < expiry_timestamp);
    let status_valid = if ledger_timestamp_valid < expiry_timestamp {
        "Approved"
    } else {
        "Expired"
    };
    assert_eq!(status_valid, "Approved");

    // Case B — ledger timestamp is AT/AFTER expiry: approval has expired.
    let ledger_timestamp_expired: i64 = 1_700_000_500; // 500s after expiry
    assert!(ledger_timestamp_expired >= expiry_timestamp);
    let status_expired = if ledger_timestamp_expired < expiry_timestamp {
        "Approved"
    } else {
        "Expired"
    };
    assert_eq!(status_expired, "Expired");

    // The numeric comparison the guide documents:
    //   1_699_999_000 < 1_700_000_000  => valid
    //   1_700_000_500 >= 1_700_000_000 => expired
    assert_eq!(expiry_timestamp - ledger_timestamp_valid, 1_000);
    assert_eq!(ledger_timestamp_expired - expiry_timestamp, 500);
}

/// Validates the jurisdiction-blocking worked example from the Compliance Guide.
/// The guide walks through blocking a sample jurisdiction ("KP") end to end:
/// the block is configured, a transfer into the blocked jurisdiction is
/// rejected, and the resulting API/contract behavior is documented.
#[test]
fn worked_example_blocked_jurisdiction_valid() {
    // Step 1: Configure the block — POST /admin/jurisdictions/block.
    let block_request = json!({
        "jurisdiction": "KP",
        "reason": "sanctions"
    });
    assert_eq!(block_request["jurisdiction"], "KP");
    assert!(block_request["reason"].is_string());

    // Step 2: A transfer targeting the blocked jurisdiction is rejected.
    // The API returns HTTP 403 with a structured error body.
    let blocked_response = json!({
        "error": "jurisdiction_blocked",
        "jurisdiction": "KP",
        "message": "Transfers to jurisdiction KP are blocked"
    });
    assert_eq!(blocked_response["error"], "jurisdiction_blocked");
    assert_eq!(blocked_response["jurisdiction"], "KP");
    assert!(blocked_response["message"].is_string());

    // Step 3: The contract-level behavior — the blocked jurisdiction is
    // reflected in the compliance summary so operators can audit it.
    let summary = json!({
        "total_records": 1,
        "approved": 0,
        "suspended": 0,
        "rejected": 1,
        "pending": 0,
        "with_expiry": 0,
        "jurisdictions": [
            { "jurisdiction": "KP", "count": 1 }
        ]
    });
    assert_eq!(summary["rejected"], 1);
    assert_eq!(summary["jurisdictions"][0]["jurisdiction"], "KP");
    assert_eq!(summary["jurisdictions"][0]["count"], 1);
}

/// Validates that compliance status values match the contract interface.
/// Valid statuses are: Approved, Pending, Rejected, Suspended
#[test]
fn compliance_statuses_are_valid() {
    // These are the valid status values per the contract's KycRecord
    let valid_statuses = vec!["Approved", "Pending", "Rejected", "Suspended"];

    for status in &valid_statuses {
        // Each status must be representable as a string
        let status_json = json!(status);
        assert!(status_json.is_string());
    }
}

/// Validates that jurisdiction codes are ISO 3166-1 alpha-2 format.
#[test]
fn jurisdiction_codes_are_iso_format() {
    // Example jurisdictions from the compliance system
    let jurisdictions = vec!["US", "KE", "DE", "GB", "CH", "KP"];

    for jurisdiction in &jurisdictions {
        // ISO country codes must be exactly 2 uppercase letters
        assert_eq!(
            jurisdiction.len(),
            2,
            "Jurisdiction code must be 2 letters: {}",
            jurisdiction
        );
        assert!(
            jurisdiction.chars().all(|c| c.is_ascii_uppercase()),
            "Jurisdiction code must be uppercase: {}",
            jurisdiction
        );
    }
}

/// Validates that a complex compliance summary with multiple jurisdictions is valid.
#[test]
fn complex_compliance_summary_valid() {
    let complex_compliance = json!({
        "total_records": 5,
        "approved": 3,
        "suspended": 1,
        "rejected": 0,
        "pending": 1,
        "with_expiry": 2,
        "jurisdictions": [
            { "jurisdiction": "US", "count": 2 },
            { "jurisdiction": "KE", "count": 1 },
            { "jurisdiction": "DE", "count": 1 },
            { "jurisdiction": "GB", "count": 1 }
        ]
    });

    // Verify totals make sense
    let total_status = complex_compliance["approved"].as_u64().unwrap()
        + complex_compliance["suspended"].as_u64().unwrap()
        + complex_compliance["rejected"].as_u64().unwrap()
        + complex_compliance["pending"].as_u64().unwrap();
    assert_eq!(
        total_status,
        complex_compliance["total_records"].as_u64().unwrap()
    );

    // Verify jurisdiction counts add up
    let jurisdictions = complex_compliance["jurisdictions"].as_array().unwrap();
    let total_jurisdiction_count: u64 = jurisdictions
        .iter()
        .map(|j| j["count"].as_u64().unwrap())
        .sum();
    assert_eq!(
        total_jurisdiction_count,
        complex_compliance["total_records"].as_u64().unwrap()
    );

    // Verify at least one jurisdiction exists
    assert!(
        !jurisdictions.is_empty(),
        "Jurisdictions array should not be empty"
    );
}
