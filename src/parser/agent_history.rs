use serde::Deserialize;

use super::ParseError;

/// A single agent session record from agent-history.json.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentSession {
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub phase: Option<String>,
    pub plan: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    /// Extra/unknown fields preserved as serde_json::Value
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Parse an agent-history.json file content into structured data.
///
/// Handles both array format `[{...}, ...]` and object-with-array format
/// `{"sessions": [{...}, ...]}`. Unknown fields on each record are preserved
/// in the `extra` field.
pub fn parse_agent_history(content: &str) -> Result<Vec<AgentSession>, ParseError> {
    if content.trim().is_empty() {
        return Err(ParseError::InvalidContent(
            "agent-history.json content is empty".to_string(),
        ));
    }

    let raw: serde_json::Value = serde_json::from_str(content)?;

    match raw {
        serde_json::Value::Array(arr) => {
            let sessions: Vec<AgentSession> = arr
                .into_iter()
                .map(|v| serde_json::from_value(v))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(sessions)
        }
        serde_json::Value::Object(ref map) => {
            // Try "sessions" key, then "history" key
            let arr = map
                .get("sessions")
                .or_else(|| map.get("history"))
                .and_then(|v| v.as_array());

            match arr {
                Some(arr) => {
                    let sessions: Vec<AgentSession> = arr
                        .iter()
                        .cloned()
                        .map(|v| serde_json::from_value(v))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(sessions)
                }
                None => Err(ParseError::InvalidContent(
                    "agent-history.json object has no 'sessions' or 'history' array".to_string(),
                )),
            }
        }
        _ => Err(ParseError::InvalidContent(
            "agent-history.json must be an array or object".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string("tests/fixtures/planning/agent-history.json")
            .expect("agent-history.json fixture should exist")
    }

    #[test]
    fn test_parse_agent_history_records() {
        let sessions = parse_agent_history(&fixture()).unwrap();
        assert_eq!(sessions.len(), 3);

        let first = &sessions[0];
        assert_eq!(first.agent_id.as_deref(), Some("agent-001"));
        assert_eq!(first.agent_type.as_deref(), Some("planner"));
        assert_eq!(first.phase.as_deref(), Some("01"));
        assert_eq!(first.plan.as_deref(), Some("01"));
        assert_eq!(
            first.started_at.as_deref(),
            Some("2026-03-06T19:00:00Z")
        );
        assert_eq!(
            first.ended_at.as_deref(),
            Some("2026-03-06T19:10:00Z")
        );
    }

    #[test]
    fn test_parse_agent_history_extra_fields() {
        let sessions = parse_agent_history(&fixture()).unwrap();
        let second = &sessions[1];
        // The "extra_field" should be in the extra Value
        assert_eq!(
            second.extra.get("extra_field").and_then(|v| v.as_str()),
            Some("some_custom_data")
        );
    }

    #[test]
    fn test_parse_agent_history_missing_optional_fields() {
        let sessions = parse_agent_history(&fixture()).unwrap();
        let third = &sessions[2];
        assert_eq!(third.agent_type.as_deref(), Some("verifier"));
        // plan is missing for verifier
        assert!(third.plan.is_none());
        // ended_at is missing (still running)
        assert!(third.ended_at.is_none());
    }

    #[test]
    fn test_parse_agent_history_empty_array() {
        let input = "[]";
        let sessions = parse_agent_history(input).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_parse_agent_history_object_format() {
        let input = r#"{"sessions": [{"agent_id": "a1", "agent_type": "executor"}]}"#;
        let sessions = parse_agent_history(input).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].agent_id.as_deref(), Some("a1"));
    }

    #[test]
    fn test_parse_agent_history_empty_input() {
        let result = parse_agent_history("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_agent_history_invalid_json() {
        let result = parse_agent_history("{bad json");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::JsonError(_) => {}
            e => panic!("Expected JsonError, got: {:?}", e),
        }
    }
}
