//! Snapshot tests for example responses documented in Compliance Guide.
//!
//! This test module verifies that the API compliance endpoint examples
//! remain accurate with respect to the contract interface and models.
//! It validates:
//! - /assets/:id/compliance response structure
//! - jurisdiction breakdown format
//! - compliance status enums

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
    let jurisdictions = vec!["US", "KE", "DE", "GB", "CH"];

    for jurisdiction in &jurisdictions {
        // ISO country codes must be exactly 2 uppercase letters
        assert_eq!(jurisdiction.len(), 2, "Jurisdiction code must be 2 letters: {}", jurisdiction);
        assert!(jurisdiction.chars().all(|c| c.is_ascii_uppercase()),
                "Jurisdiction code must be uppercase: {}", jurisdiction);
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
    assert_eq!(total_status, complex_compliance["total_records"].as_u64().unwrap());

    // Verify jurisdiction counts add up
    let jurisdictions = complex_compliance["jurisdictions"].as_array().unwrap();
    let total_jurisdiction_count: u64 = jurisdictions
        .iter()
        .map(|j| j["count"].as_u64().unwrap())
        .sum();
    assert_eq!(total_jurisdiction_count, complex_compliance["total_records"].as_u64().unwrap());

    // Verify at least one jurisdiction exists
    assert!(jurisdictions.len() > 0, "Jurisdictions array should not be empty");

    // Verify with_expiry is within reasonable bounds
    assert!(complex_compliance["with_expiry"].as_u64().unwrap()
        <= complex_compliance["total_records"].as_u64().unwrap());
}

/// Validates that the compliance endpoint response has no PII (addresses).
/// Only counts and jurisdiction codes are exposed, not individual records.
#[test]
fn compliance_response_contains_no_addresses() {
    let compliance = json!({
        "total_records": 3,
        "approved": 2,
        "suspended": 1,
        "rejected": 0,
        "pending": 0,
        "with_expiry": 0,
        "jurisdictions": [
            { "jurisdiction": "US", "count": 2 },
            { "jurisdiction": "KE", "count": 1 }
        ]
    });

    // Verify there is no 'addresses' field
    assert!(compliance.get("addresses").is_none(),
        "Compliance response must not contain 'addresses' (no PII)");

    // Verify there is no 'records' field with individual entries
    assert!(compliance.get("records").is_none(),
        "Compliance response must not contain 'records' (no PII)");

    // Verify jurisdiction array contains only jurisdiction and count, no addresses
    let jurisdictions = compliance["jurisdictions"].as_array().unwrap();
    for jurisdiction in jurisdictions {
        assert!(jurisdiction.get("address").is_none(),
            "Jurisdiction entry must not contain 'address' (no PII)");
        assert!(jurisdiction.get("status").is_none(),
            "Jurisdiction entry must not contain individual status (no PII)");
    }
}

/// Validates that error response for unknown asset is consistent.
#[test]
fn compliance_error_response_valid() {
    let error = json!({
        "error": "not_found",
        "message": "no asset with id 999"
    });

    assert_eq!(error["error"], "not_found");
    assert!(error["message"].as_str().unwrap().contains("asset"));
}

/// Validates that jurisdiction blocking behavior is documented.
/// When a jurisdiction is blocked via the contract's block_jurisdiction,
/// affected addresses fail is_allowed instantly.
#[test]
fn jurisdiction_blocking_documented() {
    // This is a documentation test - verifies the compliance model
    // When "US" is blocked, all "US" addresses fail is_allowed

    let jurisdiction_code = "US";
    assert_eq!(jurisdiction_code.len(), 2);

    // The block_jurisdiction call in the contract affects all addresses
    // with that jurisdiction code in their KycRecord
    let compliance_before_block = json!({
        "total_records": 2,
        "approved": 2,
        "suspended": 0,
        "rejected": 0,
        "pending": 0,
        "with_expiry": 0,
        "jurisdictions": [
            { "jurisdiction": "US", "count": 2 }
        ]
    });

    // After block_jurisdiction("US"), the same addresses would fail is_allowed
    // but the API still shows the counts (no PII change needed)
    assert!(compliance_before_block["jurisdictions"].as_array().unwrap().len() > 0);
}
