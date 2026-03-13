//! Report generation for test results.
//!
//! Supports JUnit XML and JSON output formats for CI integration.

use crate::runner::CollectionRunResult;

/// Generate a JUnit XML report from collection run results.
pub fn to_junit_xml(result: &CollectionRunResult) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

    let failures: usize = result
        .request_results
        .iter()
        .map(|r| r.tests.iter().filter(|t| !t.passed).count())
        .sum();
    let total: usize = result.request_results.iter().map(|r| r.tests.len()).sum();

    xml.push_str(&format!(
        "<testsuites name=\"{}\" tests=\"{}\" failures=\"{}\" time=\"{:.3}\">\n",
        escape_xml(&result.collection_name),
        total,
        failures,
        result.total_duration_ms as f64 / 1000.0,
    ));

    for req_result in &result.request_results {
        let req_failures = req_result.tests.iter().filter(|t| !t.passed).count();
        xml.push_str(&format!(
            "  <testsuite name=\"{} {}\" tests=\"{}\" failures=\"{}\">\n",
            escape_xml(&req_result.method),
            escape_xml(&req_result.name),
            req_result.tests.len(),
            req_failures,
        ));

        for test in &req_result.tests {
            if test.passed {
                xml.push_str(&format!(
                    "    <testcase name=\"{}\" />\n",
                    escape_xml(&test.name),
                ));
            } else {
                xml.push_str(&format!(
                    "    <testcase name=\"{}\">\n",
                    escape_xml(&test.name),
                ));
                let msg = test.error.as_deref().unwrap_or("Test failed");
                xml.push_str(&format!(
                    "      <failure message=\"{}\">{}</failure>\n",
                    escape_xml(msg),
                    escape_xml(msg),
                ));
                xml.push_str("    </testcase>\n");
            }
        }

        if let Some(ref err) = req_result.error {
            xml.push_str(&format!(
                "    <testcase name=\"Request execution\">\n      <error message=\"{}\">{}</error>\n    </testcase>\n",
                escape_xml(err),
                escape_xml(err),
            ));
        }

        xml.push_str("  </testsuite>\n");
    }

    xml.push_str("</testsuites>\n");
    xml
}

/// Generate a JSON report from collection run results.
pub fn to_json(result: &CollectionRunResult) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(result)
}

/// Generate a compact single-line JSON report.
pub fn to_json_compact(result: &CollectionRunResult) -> Result<String, serde_json::Error> {
    serde_json::to_string(result)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{CollectionRunResult, RequestRunResult, TestResultEntry};

    fn sample_result() -> CollectionRunResult {
        CollectionRunResult {
            collection_name: "My API Tests".to_string(),
            request_results: vec![
                RequestRunResult {
                    name: "Get Users".to_string(),
                    url: "https://api.example.com/users".to_string(),
                    method: "GET".to_string(),
                    status: Some(200),
                    duration_ms: 150,
                    tests: vec![
                        TestResultEntry {
                            name: "Status is 200".to_string(),
                            passed: true,
                            error: None,
                        },
                        TestResultEntry {
                            name: "Has users array".to_string(),
                            passed: true,
                            error: None,
                        },
                    ],
                    error: None,
                    logs: vec![],
                },
                RequestRunResult {
                    name: "Create User".to_string(),
                    url: "https://api.example.com/users".to_string(),
                    method: "POST".to_string(),
                    status: Some(400),
                    duration_ms: 80,
                    tests: vec![TestResultEntry {
                        name: "Status is 201".to_string(),
                        passed: false,
                        error: Some("Expected 201 but got 400".to_string()),
                    }],
                    error: None,
                    logs: vec![],
                },
            ],
            total_duration_ms: 230,
            total_tests: 3,
            passed_tests: 2,
            failed_tests: 1,
            timestamp: "2026-03-13T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_junit_xml_structure() {
        let xml = to_junit_xml(&sample_result());
        assert!(xml.starts_with("<?xml version=\"1.0\""));
        assert!(xml.contains("<testsuites name=\"My API Tests\""));
        assert!(xml.contains("tests=\"3\""));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("<testsuite name=\"GET Get Users\""));
        assert!(xml.contains("<testsuite name=\"POST Create User\""));
    }

    #[test]
    fn test_junit_xml_passing_test() {
        let xml = to_junit_xml(&sample_result());
        assert!(xml.contains("<testcase name=\"Status is 200\" />"));
    }

    #[test]
    fn test_junit_xml_failing_test() {
        let xml = to_junit_xml(&sample_result());
        assert!(xml.contains("<failure message=\"Expected 201 but got 400\">"));
    }

    #[test]
    fn test_junit_xml_escaping() {
        let mut result = sample_result();
        result.collection_name = "Tests & <More>".to_string();
        let xml = to_junit_xml(&result);
        assert!(xml.contains("Tests &amp; &lt;More&gt;"));
    }

    #[test]
    fn test_json_report() {
        let json = to_json(&sample_result()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["collection_name"], "My API Tests");
        assert_eq!(parsed["total_tests"], 3);
        assert_eq!(parsed["failed_tests"], 1);
    }

    #[test]
    fn test_json_compact() {
        let json = to_json_compact(&sample_result()).unwrap();
        assert!(!json.contains('\n'));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["passed_tests"], 2);
    }

    #[test]
    fn test_request_error_in_junit() {
        let result = CollectionRunResult {
            collection_name: "Error Test".to_string(),
            request_results: vec![RequestRunResult {
                name: "Bad Request".to_string(),
                url: "https://invalid".to_string(),
                method: "GET".to_string(),
                status: None,
                duration_ms: 0,
                tests: vec![],
                error: Some("Connection refused".to_string()),
                logs: vec![],
            }],
            total_duration_ms: 0,
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            timestamp: "2026-03-13T00:00:00Z".to_string(),
        };
        let xml = to_junit_xml(&result);
        assert!(xml.contains("<error message=\"Connection refused\">"));
    }
}
