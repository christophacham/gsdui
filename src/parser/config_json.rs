use serde::Deserialize;
use tracing::warn;

use super::ParseError;

/// Known fields in the GSD config.json file.
#[derive(Debug, Clone, Deserialize)]
pub struct GsdConfig {
    pub mode: Option<String>,
    pub granularity: Option<String>,
    pub parallelization: Option<bool>,
    pub commit_docs: Option<bool>,
    pub model_profile: Option<String>,
    pub workflow: Option<serde_json::Value>,
    pub planning: Option<serde_json::Value>,
    pub gates: Option<serde_json::Value>,
    pub safety: Option<serde_json::Value>,
    /// The complete raw config, preserving all fields including unknown ones
    #[serde(skip)]
    pub raw: serde_json::Value,
}

/// Known top-level field names in config.json.
const KNOWN_FIELDS: &[&str] = &[
    "mode",
    "granularity",
    "parallelization",
    "commit_docs",
    "model_profile",
    "workflow",
    "planning",
    "gates",
    "safety",
];

/// Parse a config.json file content into structured data.
///
/// Per user strict mode: unknown top-level fields generate warnings (logged),
/// but parsing still succeeds. The raw JSON is preserved for access to any field.
pub fn parse_config_json(content: &str) -> Result<GsdConfig, ParseError> {
    if content.trim().is_empty() {
        return Err(ParseError::InvalidContent(
            "config.json content is empty".to_string(),
        ));
    }

    // Parse as raw Value first to preserve all fields
    let raw: serde_json::Value = serde_json::from_str(content)?;

    // Check for unknown fields (strict mode)
    if let serde_json::Value::Object(ref map) = raw {
        for key in map.keys() {
            if !KNOWN_FIELDS.contains(&key.as_str()) {
                warn!(
                    field = key.as_str(),
                    "config.json contains unknown field (strict mode warning)"
                );
            }
        }
    }

    // Parse into typed struct (unknown fields are silently ignored by serde)
    let mut config: GsdConfig = serde_json::from_str(content)?;
    config.raw = raw;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string("tests/fixtures/planning/config.json")
            .expect("config.json fixture should exist")
    }

    #[test]
    fn test_parse_config_json_known_fields() {
        let config = parse_config_json(&fixture()).unwrap();
        assert_eq!(config.mode.as_deref(), Some("yolo"));
        assert_eq!(config.granularity.as_deref(), Some("coarse"));
        assert_eq!(config.parallelization, Some(true));
        assert_eq!(config.commit_docs, Some(true));
        assert_eq!(config.model_profile.as_deref(), Some("quality"));
    }

    #[test]
    fn test_parse_config_json_workflow() {
        let config = parse_config_json(&fixture()).unwrap();
        let workflow = config.workflow.expect("workflow should exist");
        assert_eq!(
            workflow.get("research").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            workflow.get("auto_advance").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn test_parse_config_json_nested_objects() {
        let config = parse_config_json(&fixture()).unwrap();
        let planning = config.planning.expect("planning should exist");
        assert_eq!(
            planning.get("max_plans_per_phase").and_then(|v| v.as_i64()),
            Some(5)
        );

        let gates = config.gates.expect("gates should exist");
        assert_eq!(
            gates.get("require_verification").and_then(|v| v.as_bool()),
            Some(true)
        );

        let safety = config.safety.expect("safety should exist");
        assert_eq!(
            safety
                .get("backup_before_destructive")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn test_parse_config_json_preserves_unknown_fields() {
        let config = parse_config_json(&fixture()).unwrap();
        // Unknown field "custom_field" should be in raw
        let custom = config.raw.get("custom_field");
        assert!(custom.is_some(), "Unknown field should be preserved in raw");
        assert_eq!(custom.unwrap().as_str(), Some("preserved_for_strict_mode"));
    }

    #[test]
    fn test_parse_config_json_empty_input() {
        let result = parse_config_json("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_config_json_invalid_json() {
        let result = parse_config_json("{invalid json");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::JsonError(_) => {}
            e => panic!("Expected JsonError, got: {:?}", e),
        }
    }

    #[test]
    fn test_parse_config_json_minimal() {
        let input = r#"{"mode": "safe"}"#;
        let config = parse_config_json(input).unwrap();
        assert_eq!(config.mode.as_deref(), Some("safe"));
        assert!(config.granularity.is_none());
        assert!(config.workflow.is_none());
    }
}
