use super::*;

#[test]
fn usage_scan_request_and_delete_result_roundtrip() {
    let request = UsageScanRequest {
        scan_id: "scan-1".into(),
        top_files_per_category: 20,
        show_all_files: false,
        parallelism_preset: UsageScanParallelismPreset::Balanced,
    };
    let json = serde_json::to_string(&request).expect("serialize request");
    let parsed: UsageScanRequest = serde_json::from_str(&json).expect("parse request");
    assert_eq!(parsed.scan_id, "scan-1");
    assert_eq!(
        parsed.parallelism_preset,
        UsageScanParallelismPreset::Balanced
    );

    let result = UsageDeleteResult {
        deleted: vec!["/tmp/a".into()],
        failed: vec![UsageDeleteFailure {
            path: "/tmp/b".into(),
            reason: "permission denied".into(),
        }],
    };
    let json = serde_json::to_string(&result).expect("serialize result");
    let parsed: UsageDeleteResult = serde_json::from_str(&json).expect("parse result");
    assert_eq!(parsed.deleted.len(), 1);
    assert_eq!(parsed.failed.len(), 1);
}
