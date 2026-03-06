use regex::Regex;

use super::ParseError;

/// A single plan list item from a phase's plan checklist.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanListItem {
    /// Filename of the plan (e.g., "01-01-PLAN.md")
    pub filename: String,
    /// Description text after the filename
    pub description: String,
    /// Whether the checklist item is marked complete [x]
    pub completed: bool,
}

/// A phase entry parsed from ROADMAP.md.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseEntry {
    /// Phase number as string (supports decimal: "1", "2.1")
    pub number: String,
    /// Phase name
    pub name: String,
    /// Goal description
    pub goal: Option<String>,
    /// Dependencies (e.g., ["Phase 1"])
    pub depends_on: Vec<String>,
    /// Requirement IDs (e.g., ["STATE-01", "STATE-02"])
    pub requirements: Vec<String>,
    /// Number of plans stated in the heading
    pub plan_count: Option<i64>,
    /// Individual plan checklist items
    pub plans: Vec<PlanListItem>,
    /// Whether the phase is marked complete in progress table
    pub completed: bool,
}

/// Complete parsed data from a ROADMAP.md file.
#[derive(Debug, Clone)]
pub struct RoadmapData {
    /// All phase entries, sorted by phase number
    pub phases: Vec<PhaseEntry>,
    /// Execution order description
    pub execution_order: Option<String>,
}

/// Parse a ROADMAP.md file content into structured data.
pub fn parse_roadmap(content: &str) -> Result<RoadmapData, ParseError> {
    if content.trim().is_empty() {
        return Err(ParseError::InvalidContent(
            "ROADMAP.md content is empty".to_string(),
        ));
    }

    let phases = parse_phase_entries(content)?;
    let execution_order = parse_execution_order(content);

    Ok(RoadmapData {
        phases,
        execution_order,
    })
}

/// Parse all phase entries from the Phase Details section.
fn parse_phase_entries(content: &str) -> Result<Vec<PhaseEntry>, ParseError> {
    let phase_heading_re = Regex::new(r"### Phase (\d+(?:\.\d+)?): (.+)")
        .map_err(|e| ParseError::RegexError(e.to_string()))?;

    let mut phases = Vec::new();

    // Split content by phase headings
    let heading_matches: Vec<_> = phase_heading_re
        .captures_iter(content)
        .map(|cap| {
            let full = cap.get(0).unwrap();
            let number = cap.get(1).unwrap().as_str().to_string();
            let name = cap.get(2).unwrap().as_str().trim().to_string();
            (full.start(), full.end(), number, name)
        })
        .collect();

    for (i, (_start, end, number, name)) in heading_matches.iter().enumerate() {
        // Get the section content between this heading and the next
        let section_end = if i + 1 < heading_matches.len() {
            heading_matches[i + 1].0
        } else {
            // Look for "## Progress" or end of content
            content[*end..]
                .find("\n## ")
                .map(|pos| *end + pos)
                .unwrap_or(content.len())
        };

        let section = &content[*end..section_end];
        let entry = parse_single_phase(number, name, section)?;
        phases.push(entry);
    }

    // Sort by phase number (numeric comparison supporting decimals)
    phases.sort_by(|a, b| {
        let a_num: f64 = a.number.parse().unwrap_or(0.0);
        let b_num: f64 = b.number.parse().unwrap_or(0.0);
        a_num.partial_cmp(&b_num).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(phases)
}

/// Parse a single phase section for its metadata.
fn parse_single_phase(
    number: &str,
    name: &str,
    section: &str,
) -> Result<PhaseEntry, ParseError> {
    let goal = extract_bold_field(section, "Goal");
    let depends_on = extract_bold_field(section, "Depends on")
        .map(|d| {
            d.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s.to_lowercase() != "nothing (first phase)")
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let requirements = extract_bold_field(section, "Requirements")
        .map(|r| {
            r.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let plan_count = extract_bold_field(section, "Plans")
        .and_then(|p| {
            // Extract just the number from "4 plans" or "TBD"
            p.split_whitespace()
                .next()
                .and_then(|n| n.parse::<i64>().ok())
        });

    let plans = parse_plan_checklist(section);

    Ok(PhaseEntry {
        number: number.to_string(),
        name: name.to_string(),
        goal,
        depends_on,
        requirements,
        plan_count,
        plans,
        completed: false, // Will be updated from progress table if needed
    })
}

/// Extract a **Field**: value pattern from section text.
/// Handles both `**Field**:` and `**Field:**` formats.
fn extract_bold_field(section: &str, field_name: &str) -> Option<String> {
    // Pattern 1: **Field**: value (colon outside bold)
    let pattern1 = format!("**{}**:", field_name);
    // Pattern 2: **Field:** value (colon inside bold)
    let pattern2 = format!("**{}:**", field_name);

    for line in section.lines() {
        let trimmed = line.trim();
        for pattern in [&pattern1, &pattern2] {
            if let Some(pos) = trimmed.find(pattern.as_str()) {
                let after = &trimmed[pos + pattern.len()..];
                let value = after.trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Parse plan checklist items: `- [x] 01-01-PLAN.md -- Description`
fn parse_plan_checklist(section: &str) -> Vec<PlanListItem> {
    let plan_re = Regex::new(r"- \[([ xX])\] (.+?)(?:\s+--\s+(.+))?$").unwrap();

    let mut items = Vec::new();
    let mut in_plans = false;

    for line in section.lines() {
        let trimmed = line.trim();

        // Start collecting after "Plans:" line
        if trimmed.starts_with("Plans:") || trimmed == "Plans" {
            in_plans = true;
            continue;
        }

        // Stop at next section heading
        if in_plans && trimmed.starts_with('#') {
            break;
        }

        if in_plans {
            if let Some(cap) = plan_re.captures(trimmed) {
                let completed = cap.get(1).map(|m| m.as_str() != " ").unwrap_or(false);
                let filename_desc = cap.get(2).unwrap().as_str().trim().to_string();
                let description = cap.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

                items.push(PlanListItem {
                    filename: filename_desc,
                    description,
                    completed,
                });
            }
        }
    }

    items
}

/// Extract execution order from the Progress section.
fn parse_execution_order(content: &str) -> Option<String> {
    let re = Regex::new(r"(?i)\*\*Execution Order:\*\*\s*\n(.+)").unwrap();
    re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string("tests/fixtures/planning/ROADMAP.md")
            .expect("ROADMAP.md fixture should exist")
    }

    #[test]
    fn test_parse_roadmap_phases() {
        let data = parse_roadmap(&fixture()).unwrap();
        assert!(data.phases.len() >= 3, "Should have at least 3 phases");

        let phase1 = &data.phases[0];
        assert_eq!(phase1.number, "1");
        assert_eq!(phase1.name, "Backend Foundation and State Pipeline");
        assert!(phase1.goal.is_some());
        assert!(phase1.depends_on.is_empty()); // "Nothing (first phase)" is filtered
    }

    #[test]
    fn test_parse_roadmap_decimal_phases() {
        let data = parse_roadmap(&fixture()).unwrap();

        // Find phase 2.1
        let phase_2_1 = data.phases.iter().find(|p| p.number == "2.1");
        assert!(phase_2_1.is_some(), "Should find Phase 2.1");
        let phase_2_1 = phase_2_1.unwrap();
        assert_eq!(phase_2_1.name, "Hotfix Sprint");

        // Verify sort order: 1, 2, 2.1, 3
        let numbers: Vec<&str> = data.phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, vec!["1", "2", "2.1", "3"]);
    }

    #[test]
    fn test_parse_roadmap_plan_checklist() {
        let data = parse_roadmap(&fixture()).unwrap();
        let phase1 = &data.phases[0];

        assert!(!phase1.plans.is_empty(), "Phase 1 should have plan items");
        // First plan should be completed
        let first_plan = &phase1.plans[0];
        assert!(first_plan.completed, "First plan should be completed");
        assert!(first_plan.filename.contains("01-01-PLAN.md"));
    }

    #[test]
    fn test_parse_roadmap_requirements() {
        let data = parse_roadmap(&fixture()).unwrap();
        let phase1 = &data.phases[0];
        assert!(!phase1.requirements.is_empty());
        assert!(phase1.requirements.contains(&"STATE-01".to_string()));
    }

    #[test]
    fn test_parse_roadmap_plan_count() {
        let data = parse_roadmap(&fixture()).unwrap();
        let phase1 = &data.phases[0];
        assert_eq!(phase1.plan_count, Some(4));
    }

    #[test]
    fn test_parse_roadmap_execution_order() {
        let data = parse_roadmap(&fixture()).unwrap();
        assert!(data.execution_order.is_some());
        assert!(data.execution_order.unwrap().contains("1 -> 2"));
    }

    #[test]
    fn test_parse_roadmap_empty_input() {
        let result = parse_roadmap("");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidContent(_) => {}
            e => panic!("Expected InvalidContent, got: {:?}", e),
        }
    }

    #[test]
    fn test_parse_roadmap_no_phases() {
        let input = "# Roadmap\n\nJust an overview, no phase details.";
        let data = parse_roadmap(input).unwrap();
        assert!(data.phases.is_empty());
    }
}
