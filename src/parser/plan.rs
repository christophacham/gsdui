use serde::Deserialize;

use super::frontmatter;
use super::ParseError;

/// Frontmatter fields from a PLAN.md file.
#[derive(Debug, Clone, Deserialize)]
struct PlanFrontmatter {
    phase: Option<String>,
    plan: Option<serde_json::Value>,
    #[serde(rename = "type")]
    plan_type: Option<String>,
    wave: Option<serde_json::Value>,
    depends_on: Option<Vec<String>>,
    files_modified: Option<Vec<String>>,
    autonomous: Option<bool>,
    requirements: Option<Vec<String>>,
    must_haves: Option<serde_json::Value>,
}

/// Complete parsed data from a PLAN.md file.
#[derive(Debug, Clone)]
pub struct PlanMdData {
    /// Phase identifier (e.g., "01-backend-foundation")
    pub phase: Option<String>,
    /// Plan number
    pub plan: Option<i64>,
    /// Plan type (e.g., "standard", "tdd")
    pub plan_type: Option<String>,
    /// Wave number for parallel execution
    pub wave: Option<i64>,
    /// Dependencies on other plans
    pub depends_on: Vec<String>,
    /// List of files this plan modifies
    pub files_modified: Vec<String>,
    /// Whether execution is autonomous
    pub autonomous: bool,
    /// Requirement IDs this plan addresses
    pub requirements: Vec<String>,
    /// Must-have criteria (preserved as JSON value)
    pub must_haves: Option<serde_json::Value>,
    /// Body content after frontmatter
    pub body: String,
}

/// Parse a PLAN.md file content into structured data.
pub fn parse_plan_md(content: &str) -> Result<PlanMdData, ParseError> {
    let doc = frontmatter::parse_frontmatter::<PlanFrontmatter>(content)?;
    let fm = doc.metadata;

    let plan = fm.plan.and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    });

    let wave = fm.wave.and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    });

    Ok(PlanMdData {
        phase: fm.phase,
        plan,
        plan_type: fm.plan_type,
        wave,
        depends_on: fm.depends_on.unwrap_or_default(),
        files_modified: fm.files_modified.unwrap_or_default(),
        autonomous: fm.autonomous.unwrap_or(false),
        requirements: fm.requirements.unwrap_or_default(),
        must_haves: fm.must_haves,
        body: doc.content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string("tests/fixtures/planning/phases/01-test-phase/01-01-PLAN.md")
            .expect("PLAN.md fixture should exist")
    }

    #[test]
    fn test_parse_plan_md_frontmatter() {
        let data = parse_plan_md(&fixture()).unwrap();
        assert_eq!(data.phase.as_deref(), Some("01-backend-foundation"));
        assert_eq!(data.plan, Some(1));
        assert_eq!(data.plan_type.as_deref(), Some("standard"));
        assert_eq!(data.wave, Some(1));
        assert!(data.autonomous);
    }

    #[test]
    fn test_parse_plan_md_arrays() {
        let data = parse_plan_md(&fixture()).unwrap();
        assert_eq!(data.depends_on, vec!["none"]);
        assert!(!data.files_modified.is_empty());
        assert!(data.files_modified.contains(&"src/main.rs".to_string()));
        assert_eq!(
            data.requirements,
            vec!["STATE-11", "STATE-12", "INFRA-01"]
        );
    }

    #[test]
    fn test_parse_plan_md_must_haves() {
        let data = parse_plan_md(&fixture()).unwrap();
        assert!(data.must_haves.is_some());
        let must_haves = data.must_haves.unwrap();
        assert!(must_haves.get("truths").is_some());
        assert!(must_haves.get("artifacts").is_some());
    }

    #[test]
    fn test_parse_plan_md_body() {
        let data = parse_plan_md(&fixture()).unwrap();
        assert!(data.body.contains("Plan 01-01"));
    }

    #[test]
    fn test_parse_plan_md_empty_input() {
        let result = parse_plan_md("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_plan_md_minimal() {
        let input = "---\nphase: test\nplan: 5\n---\nMinimal plan.";
        let data = parse_plan_md(input).unwrap();
        assert_eq!(data.phase.as_deref(), Some("test"));
        assert_eq!(data.plan, Some(5));
        assert!(!data.autonomous);
        assert!(data.depends_on.is_empty());
        assert!(data.requirements.is_empty());
    }
}
