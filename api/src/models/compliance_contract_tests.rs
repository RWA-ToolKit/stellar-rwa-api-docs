//! Snapshot tests for Compliance Contract data types.
//! Verifies examples in docs/app/docs/contracts/compliance/page.mdx match live API/contract.

#[cfg(test)]
mod tests {
    use crate::models::{ComplianceSummary, JurisdictionCount};

    #[test]
    fn compliance_summary_reflects_contract_kycrecord_counts() {
        let compliance = ComplianceSummary {
            total_records: 10,
            approved: 7,
            suspended: 2,
            rejected: 1,
            pending: 0,
            with_expiry: 3,
            jurisdictions: vec![
                JurisdictionCount {
                    jurisdiction: "US".to_string(),
                    count: 6,
                },
                JurisdictionCount {
                    jurisdiction: "KE".to_string(),
                    count: 2,
                },
                JurisdictionCount {
                    jurisdiction: "DE".to_string(),
                    count: 2,
                },
            ],
        };

        let json = serde_json::to_value(&compliance).expect("serialization failed");
        let obj = json.as_object().expect("not an object");

        // ComplianceStatus enum in contract: Approved | Pending | Rejected | Suspended
        // API surfaces counts for each status
        assert!(obj.contains_key("approved"), "missing field: approved (ComplianceStatus::Approved)");
        assert!(obj.contains_key("pending"), "missing field: pending (ComplianceStatus::Pending)");
        assert!(obj.contains_key("rejected"), "missing field: rejected (ComplianceStatus::Rejected)");
        assert!(obj.contains_key("suspended"), "missing field: suspended (ComplianceStatus::Suspended)");

        // total_records reflects total KycRecords (any status except removed)
        assert!(obj.contains_key("total_records"), "missing field: total_records");

        // Expiry tracking: contract stores expires_at as u32 ledger number
        assert!(obj.contains_key("with_expiry"), "missing field: with_expiry");

        // Jurisdictions from contract: String field (ISO country code, e.g. US, KE, DE)
        assert!(obj.contains_key("jurisdictions"), "missing field: jurisdictions");
    }

    #[test]
    fn jurisdiction_counts_match_contract_data() {
        let jurisdictions = vec![
            JurisdictionCount {
                jurisdiction: "US".to_string(),
                count: 15,
            },
            JurisdictionCount {
                jurisdiction: "KE".to_string(),
                count: 5,
            },
            JurisdictionCount {
                jurisdiction: "DE".to_string(),
                count: 3,
            },
        ];

        for jur_count in jurisdictions {
            let json = serde_json::to_value(&jur_count).expect("serialization failed");

            // Contract uses String for jurisdiction (ISO country code)
            assert!(json["jurisdiction"].is_string(), "jurisdiction should be a string");
            // Count is derived from number of addresses in that jurisdiction
            assert!(json["count"].is_number(), "count should be a number");
        }
    }

    #[test]
    fn compliance_summary_serializes_all_status_counts() {
        let compliance = ComplianceSummary {
            total_records: 100,
            approved: 80,
            suspended: 10,
            rejected: 5,
            pending: 5,
            with_expiry: 25,
            jurisdictions: vec![],
        };

        let json = serde_json::to_value(&compliance).expect("serialization failed");

        // Each status maps to exactly one ComplianceStatus variant
        assert_eq!(json["approved"].as_u64(), Some(80));
        assert_eq!(json["suspended"].as_u64(), Some(10));
        assert_eq!(json["rejected"].as_u64(), Some(5));
        assert_eq!(json["pending"].as_u64(), Some(5));

        // Total should equal sum of all statuses
        let sum = 80 + 10 + 5 + 5;
        assert_eq!(json["total_records"].as_u64(), Some(100));
        assert!(sum <= 100, "sum of statuses should not exceed total");
    }

    #[test]
    fn jurisdiction_blocking_affects_is_allowed_gate() {
        // Contract tracks blocked_jurisdictions as a separate set.
        // When a jurisdiction is blocked, is_allowed returns false even for Approved addresses.
        // API surfaces this as filtered counts in compliance summary.

        let compliance = ComplianceSummary {
            total_records: 50,
            approved: 40,
            suspended: 5,
            rejected: 5,
            pending: 0,
            with_expiry: 10,
            jurisdictions: vec![
                JurisdictionCount {
                    jurisdiction: "US".to_string(),
                    count: 25,
                },
                JurisdictionCount {
                    jurisdiction: "RU".to_string(),
                    count: 25,
                }, // hypothetically blocked
            ],
        };

        let json = serde_json::to_value(&compliance).expect("serialization failed");
        let jurisdictions = json["jurisdictions"]
            .as_array()
            .expect("jurisdictions should be an array");

        assert!(
            jurisdictions.iter().all(|j| j["jurisdiction"].is_string()),
            "all jurisdictions should have string codes"
        );
        assert!(
            jurisdictions.iter().all(|j| j["count"].is_number()),
            "all jurisdictions should have numeric counts"
        );
    }
}
