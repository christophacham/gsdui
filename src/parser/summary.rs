use regex::Regex;
use serde::Deserialize;

use super::frontmatter;
use super::ParseError;

/// A single commit record parsed from the Task Commits section.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitRecord {
    /// Task number (1-based)
    pub task_number: i64,
    /// Task name/description
    pub task_name: String,
    /// Git commit hash (short form)
    pub commit_hash: Option<String>,
    /// Commit message (not always present in SUMMARY format)
    pub commit_message: Option<String>,
    /// Commit type: feat, fix, test, refactor, docs, chore
    pub commit_type: Option<String>,
}

/// Frontmatter fields from SUMMARY.md.
#[derive(Debug, Clone, Deserialize)]
struct SummaryFrontmatter {
    phase: Option<String>,
    plan: Option<serde_json::Value>,
    duration: Option<String>,
    completed: Option<String>,
    #[serde(rename = "key-files")]
    key_files: Option<serde_json::Value>,
    #[serde(rename = "requirements-completed")]
    requirements_completed: Option<Vec<String>>,
    provides: Option<serde_json::Value>,
    affects: Option<serde_json::Value>,
}

/// Complete parsed data from a SUMMARY.md file.
#[derive(Debug, Clone)]
pub struct SummaryMdData {
    pub phase: Option<String>,
    pub plan: Option<i64>,
    pub duration: Option<String>,
    pub completed: Option<String>,
    pub key_files: Vec<String>,
    pub requirements_completed: Vec<String>,
    pub provides: Option<String>,
    pub affects: Option<String>,
    /// Commit records parsed from the body's "Task Commits" section
    pub commits: Vec<CommitRecord>,
}

/// Parse a SUMMARY.md file content into structured data.
pub fn parse_summary_md(content: &str) -> Result<SummaryMdData, ParseError> {
    let doc = frontmatter::parse_frontmatter::<SummaryFrontmatter>(content)?;
    let fm = doc.metadata;

    let plan = fm.plan.and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    });

    let key_files = extract_string_list(fm.key_files);

    let provides = fm.provides.map(|v| match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    });

    let affects = fm.affects.map(|v| match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    });

    let commits = parse_commit_section(&doc.content);

    Ok(SummaryMdData {
        phase: fm.phase,
        plan,
        duration: fm.duration,
        completed: fm.completed,
        key_files,
        requirements_completed: fm.requirements_completed.unwrap_or_default(),
        provides,
        affects,
        commits,
    })
}

/// Extract a list of strings from a serde_json::Value that may be an array or object with nested arrays.
fn extract_string_list(value: Option<serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(arr)) => arr
            .into_iter()
            .filter_map(|v| match v {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
            .collect(),
        Some(serde_json::Value::Object(obj)) => {
            // Handle key-files format: { created: [...], modified: [...] }
            let mut files = Vec::new();
            for (_key, val) in obj {
                if let serde_json::Value::Array(arr) = val {
                    for item in arr {
                        if let serde_json::Value::String(s) = item {
                            files.push(s);
                        }
                    }
                }
            }
            files
        }
        _ => Vec::new(),
    }
}

/// Parse the "## Task Commits" section from the SUMMARY body.
///
/// Expected format:
/// ```text
/// ## Task Commits
///
/// 1. **Task 1: Name here** - `abc1234` (feat)
/// 2. **Task 2: Another task** - `def5678` (fix)
/// ```
fn parse_commit_section(body: &str) -> Vec<CommitRecord> {
    // Find the Task Commits section
    let section_start = body.find("## Task Commits");
    let section = match section_start {
        Some(pos) => &body[pos..],
        None => return Vec::new(),
    };

    // Find the end of this section (next ## heading or end of body)
    let section_end = section[1..]
        .find("\n## ")
        .map(|p| p + 1)
        .unwrap_or(section.len());
    let section = &section[..section_end];

    // Pattern: N. **Task N: Name** - `hash` (type)
    let commit_re = Regex::new(
        r"(\d+)\.\s+\*\*(.+?)\*\*\s*(?:-|--)\s*`([a-f0-9]+)`\s*\((\w+)\)"
    ).unwrap();

    let mut commits = Vec::new();

    for cap in commit_re.captures_iter(section) {
        let task_number = cap.get(1).unwrap().as_str().parse::<i64>().unwrap_or(0);
        let task_name = cap.get(2).unwrap().as_str().trim().to_string();
        let commit_hash = cap.get(3).map(|m| m.as_str().to_string());
        let commit_type = cap.get(4).map(|m| m.as_str().to_string());

        commits.push(CommitRecord {
            task_number,
            task_name,
            commit_hash,
            commit_message: None,
            commit_type,
        });
    }

    commits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string("tests/fixtures/planning/phases/01-test-phase/01-01-SUMMARY.md")
            .expect("SUMMARY.md fixture should exist")
    }

    #[test]
    fn test_parse_summary_md_frontmatter() {
        let data = parse_summary_md(&fixture()).unwrap();
        assert_eq!(data.phase.as_deref(), Some("01-backend-foundation"));
        assert_eq!(data.plan, Some(1));
        assert_eq!(data.duration.as_deref(), Some("9min"));
        assert_eq!(data.completed.as_deref(), Some("2026-03-06"));
    }

    #[test]
    fn test_parse_summary_md_key_files() {
        let data = parse_summary_md(&fixture()).unwrap();
        assert!(!data.key_files.is_empty());
        assert!(data.key_files.contains(&"Cargo.toml".to_string()));
        assert!(data.key_files.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_parse_summary_md_requirements_completed() {
        let data = parse_summary_md(&fixture()).unwrap();
        assert_eq!(
            data.requirements_completed,
            vec!["STATE-11", "STATE-12", "INFRA-01"]
        );
    }

    #[test]
    fn test_parse_summary_md_commits() {
        let data = parse_summary_md(&fixture()).unwrap();
        assert_eq!(data.commits.len(), 2);

        let first = &data.commits[0];
        assert_eq!(first.task_number, 1);
        assert!(first.task_name.contains("scaffold"));
        assert_eq!(first.commit_hash.as_deref(), Some("5f0a9d2"));
        assert_eq!(first.commit_type.as_deref(), Some("feat"));

        let second = &data.commits[1];
        assert_eq!(second.task_number, 2);
        assert_eq!(second.commit_hash.as_deref(), Some("469604c"));
    }

    #[test]
    fn test_parse_summary_md_zero_commits() {
        let input = "---\nphase: test\nplan: 1\n---\n# Summary\n\nNo task commits section here.";
        let data = parse_summary_md(input).unwrap();
        assert!(data.commits.is_empty());
    }

    #[test]
    fn test_parse_summary_md_empty_input() {
        let result = parse_summary_md("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_summary_md_provides_and_affects() {
        let data = parse_summary_md(&fixture()).unwrap();
        // provides is an array in our fixture, should be joined
        assert!(data.provides.is_some());
        // affects is an array in our fixture
        assert!(data.affects.is_some());
    }
}
