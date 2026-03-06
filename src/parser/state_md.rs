use regex::Regex;
use serde::Deserialize;

use super::frontmatter;
use super::ParseError;

/// Progress data extracted from STATE.md frontmatter.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ProgressData {
    pub total_phases: Option<i64>,
    pub completed_phases: Option<i64>,
    pub total_plans: Option<i64>,
    pub completed_plans: Option<i64>,
    pub percent: Option<i64>,
}

/// Frontmatter fields from STATE.md (deserialized from YAML).
#[derive(Debug, Clone, Deserialize)]
struct StateFrontmatter {
    gsd_state_version: Option<String>,
    milestone: Option<String>,
    milestone_name: Option<String>,
    status: Option<String>,
    stopped_at: Option<String>,
    last_updated: Option<String>,
    last_activity: Option<String>,
    progress: Option<ProgressData>,
}

/// Complete parsed data from a STATE.md file.
#[derive(Debug, Clone)]
pub struct StateMdData {
    pub gsd_state_version: Option<String>,
    pub milestone: Option<String>,
    pub milestone_name: Option<String>,
    pub status: Option<String>,
    pub stopped_at: Option<String>,
    pub last_updated: Option<String>,
    pub last_activity: Option<String>,
    pub progress: Option<ProgressData>,
    /// Extracted from body: "Phase: N of M" -> "N"
    pub current_phase: Option<String>,
    /// Extracted from body: "Plan: N of M" -> "N"
    pub current_plan: Option<String>,
}

/// Parse a STATE.md file content into structured data.
///
/// Extracts YAML frontmatter fields and parses the body for Current Position section.
pub fn parse_state_md(content: &str) -> Result<StateMdData, ParseError> {
    let doc = frontmatter::parse_frontmatter::<StateFrontmatter>(content)?;
    let fm = doc.metadata;

    // Parse body for current position
    let (current_phase, current_plan) = parse_current_position(&doc.content);

    Ok(StateMdData {
        gsd_state_version: fm.gsd_state_version,
        milestone: fm.milestone,
        milestone_name: fm.milestone_name,
        status: fm.status,
        stopped_at: fm.stopped_at,
        last_updated: fm.last_updated,
        last_activity: fm.last_activity,
        progress: fm.progress,
        current_phase,
        current_plan,
    })
}

/// Extract current phase and plan numbers from the body text.
fn parse_current_position(body: &str) -> (Option<String>, Option<String>) {
    let phase_re = Regex::new(r"Phase:\s*(\d+)\s+of\s+\d+").unwrap();
    let plan_re = Regex::new(r"Plan:\s*(\d+)\s+of\s+\d+").unwrap();

    let phase = phase_re
        .captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    let plan = plan_re
        .captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    (phase, plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string("tests/fixtures/planning/STATE.md")
            .expect("STATE.md fixture should exist")
    }

    #[test]
    fn test_parse_state_md_frontmatter() {
        let data = parse_state_md(&fixture()).unwrap();
        assert_eq!(data.gsd_state_version.as_deref(), Some("1.0"));
        assert_eq!(data.milestone.as_deref(), Some("v1.0"));
        assert_eq!(data.milestone_name.as_deref(), Some("milestone"));
        assert_eq!(data.status.as_deref(), Some("executing"));
        assert_eq!(
            data.stopped_at.as_deref(),
            Some("Completed 01-01-PLAN.md")
        );
        assert_eq!(
            data.last_updated.as_deref(),
            Some("2026-03-06T19:54:33Z")
        );
        assert!(data.last_activity.is_some());
    }

    #[test]
    fn test_parse_state_md_progress() {
        let data = parse_state_md(&fixture()).unwrap();
        let progress = data.progress.expect("progress should be present");
        assert_eq!(progress.total_phases, Some(4));
        assert_eq!(progress.completed_phases, Some(0));
        assert_eq!(progress.total_plans, Some(11));
        assert_eq!(progress.completed_plans, Some(1));
        assert_eq!(progress.percent, Some(9));
    }

    #[test]
    fn test_parse_state_md_current_position() {
        let data = parse_state_md(&fixture()).unwrap();
        assert_eq!(data.current_phase.as_deref(), Some("1"));
        assert_eq!(data.current_plan.as_deref(), Some("2"));
    }

    #[test]
    fn test_parse_state_md_empty_input() {
        let result = parse_state_md("");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::NoFrontmatter => {}
            e => panic!("Expected NoFrontmatter, got: {:?}", e),
        }
    }

    #[test]
    fn test_parse_state_md_no_body_position() {
        let input = "---\nstatus: executing\n---\nNo current position section here.";
        let data = parse_state_md(input).unwrap();
        assert_eq!(data.status.as_deref(), Some("executing"));
        assert!(data.current_phase.is_none());
        assert!(data.current_plan.is_none());
    }
}
