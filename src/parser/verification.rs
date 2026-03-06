use serde::Deserialize;

use super::frontmatter;
use super::ParseError;

/// Verification status values for a phase.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    GapsFound,
    HumanNeeded,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStatus::Passed => write!(f, "passed"),
            VerificationStatus::GapsFound => write!(f, "gaps_found"),
            VerificationStatus::HumanNeeded => write!(f, "human_needed"),
        }
    }
}

/// Frontmatter fields from VERIFICATION.md.
#[derive(Debug, Clone, Deserialize)]
struct VerificationFrontmatter {
    phase: Option<String>,
    status: Option<String>,
    score: Option<String>,
    verified_at: Option<String>,
}

/// Complete parsed data from a VERIFICATION.md file.
#[derive(Debug, Clone)]
pub struct VerificationMdData {
    /// Phase identifier
    pub phase: Option<String>,
    /// Verification status
    pub status: VerificationStatus,
    /// Score in "N/M" format
    pub score: Option<String>,
    /// Timestamp of verification
    pub verified_at: Option<String>,
}

/// Parse a VERIFICATION.md file content into structured data.
pub fn parse_verification_md(content: &str) -> Result<VerificationMdData, ParseError> {
    let doc = frontmatter::parse_frontmatter::<VerificationFrontmatter>(content)?;
    let fm = doc.metadata;

    let status = match fm.status.as_deref() {
        Some("passed") => VerificationStatus::Passed,
        Some("gaps_found") => VerificationStatus::GapsFound,
        Some("human_needed") => VerificationStatus::HumanNeeded,
        Some(other) => {
            return Err(ParseError::InvalidContent(format!(
                "Unknown verification status: '{}'",
                other
            )));
        }
        None => {
            return Err(ParseError::InvalidContent(
                "Missing required 'status' field in VERIFICATION.md frontmatter".to_string(),
            ));
        }
    };

    Ok(VerificationMdData {
        phase: fm.phase,
        status,
        score: fm.score,
        verified_at: fm.verified_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string(
            "tests/fixtures/planning/phases/01-test-phase/01-VERIFICATION.md",
        )
        .expect("VERIFICATION.md fixture should exist")
    }

    #[test]
    fn test_parse_verification_md_passed() {
        let data = parse_verification_md(&fixture()).unwrap();
        assert_eq!(data.phase.as_deref(), Some("01-backend-foundation"));
        assert_eq!(data.status, VerificationStatus::Passed);
        assert_eq!(data.score.as_deref(), Some("5/5"));
        assert_eq!(
            data.verified_at.as_deref(),
            Some("2026-03-06T20:15:00Z")
        );
    }

    #[test]
    fn test_parse_verification_md_gaps_found() {
        let input = "---\nphase: test\nstatus: gaps_found\nscore: \"3/5\"\n---\nGaps found.";
        let data = parse_verification_md(input).unwrap();
        assert_eq!(data.status, VerificationStatus::GapsFound);
        assert_eq!(data.score.as_deref(), Some("3/5"));
    }

    #[test]
    fn test_parse_verification_md_human_needed() {
        let input = "---\nstatus: human_needed\nscore: \"2/5\"\n---\nNeeds human review.";
        let data = parse_verification_md(input).unwrap();
        assert_eq!(data.status, VerificationStatus::HumanNeeded);
    }

    #[test]
    fn test_parse_verification_md_missing_status() {
        let input = "---\nphase: test\nscore: \"5/5\"\n---\nNo status.";
        let result = parse_verification_md(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidContent(msg) => {
                assert!(msg.contains("status"));
            }
            e => panic!("Expected InvalidContent, got: {:?}", e),
        }
    }

    #[test]
    fn test_parse_verification_md_invalid_status() {
        let input = "---\nstatus: unknown_value\n---\nBad status.";
        let result = parse_verification_md(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidContent(msg) => {
                assert!(msg.contains("unknown_value"));
            }
            e => panic!("Expected InvalidContent, got: {:?}", e),
        }
    }

    #[test]
    fn test_parse_verification_md_empty_input() {
        let result = parse_verification_md("");
        assert!(result.is_err());
    }
}
