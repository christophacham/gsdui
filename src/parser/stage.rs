use crate::db::models::PhaseStage;

/// Derive the current stage of a phase based on which files exist in its directory.
///
/// This is a pure function: the caller passes the list of filenames found in the
/// phase directory and the expected plan count (from ROADMAP.md). No I/O is performed.
///
/// File presence is checked in priority order (later stages override earlier):
/// 1. No files -> Planned
/// 2. CONTEXT.md exists -> Discussed
/// 3. RESEARCH.md exists -> Researched
/// 4. Any PLAN.md exists -> PlannedReady
/// 5. Some SUMMARY.md exist (but not all) -> Executing
/// 6. All SUMMARY.md exist (count matches plan_count) -> Executed
/// 7. VERIFICATION.md exists -> Verified
pub fn derive_stage(files: &[String], plan_count: usize) -> PhaseStage {
    if files.is_empty() {
        return PhaseStage::Planned;
    }

    let has_context = files.iter().any(|f| f.ends_with("CONTEXT.md"));
    let has_research = files.iter().any(|f| f.ends_with("RESEARCH.md"));
    let has_plan = files.iter().any(|f| f.contains("-PLAN.md"));
    let has_verification = files.iter().any(|f| f.ends_with("VERIFICATION.md"));

    let summary_count = files.iter().filter(|f| f.contains("-SUMMARY.md")).count();

    // Priority: later stages override earlier
    if has_verification {
        return PhaseStage::Verified;
    }

    if plan_count > 0 && summary_count >= plan_count {
        return PhaseStage::Executed;
    }

    if summary_count > 0 {
        return PhaseStage::Executing;
    }

    if has_plan {
        return PhaseStage::PlannedReady;
    }

    if has_research {
        return PhaseStage::Researched;
    }

    if has_context {
        return PhaseStage::Discussed;
    }

    PhaseStage::Planned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_stage_planned_empty() {
        let stage = derive_stage(&[], 0);
        assert_eq!(stage, PhaseStage::Planned);
    }

    #[test]
    fn test_derive_stage_discussed() {
        let files = vec!["01-CONTEXT.md".to_string()];
        let stage = derive_stage(&files, 0);
        assert_eq!(stage, PhaseStage::Discussed);
    }

    #[test]
    fn test_derive_stage_researched() {
        let files = vec!["01-CONTEXT.md".to_string(), "01-RESEARCH.md".to_string()];
        let stage = derive_stage(&files, 0);
        assert_eq!(stage, PhaseStage::Researched);
    }

    #[test]
    fn test_derive_stage_planned_ready() {
        let files = vec![
            "01-CONTEXT.md".to_string(),
            "01-RESEARCH.md".to_string(),
            "01-01-PLAN.md".to_string(),
            "01-02-PLAN.md".to_string(),
        ];
        let stage = derive_stage(&files, 2);
        assert_eq!(stage, PhaseStage::PlannedReady);
    }

    #[test]
    fn test_derive_stage_executing_some_summaries() {
        let files = vec![
            "01-CONTEXT.md".to_string(),
            "01-RESEARCH.md".to_string(),
            "01-01-PLAN.md".to_string(),
            "01-02-PLAN.md".to_string(),
            "01-03-PLAN.md".to_string(),
            "01-01-SUMMARY.md".to_string(),
        ];
        let stage = derive_stage(&files, 3);
        assert_eq!(stage, PhaseStage::Executing);
    }

    #[test]
    fn test_derive_stage_executed_all_summaries() {
        let files = vec![
            "01-CONTEXT.md".to_string(),
            "01-RESEARCH.md".to_string(),
            "01-01-PLAN.md".to_string(),
            "01-02-PLAN.md".to_string(),
            "01-01-SUMMARY.md".to_string(),
            "01-02-SUMMARY.md".to_string(),
        ];
        let stage = derive_stage(&files, 2);
        assert_eq!(stage, PhaseStage::Executed);
    }

    #[test]
    fn test_derive_stage_verified() {
        let files = vec![
            "01-CONTEXT.md".to_string(),
            "01-RESEARCH.md".to_string(),
            "01-01-PLAN.md".to_string(),
            "01-01-SUMMARY.md".to_string(),
            "01-VERIFICATION.md".to_string(),
        ];
        let stage = derive_stage(&files, 1);
        assert_eq!(stage, PhaseStage::Verified);
    }

    #[test]
    fn test_derive_stage_verified_overrides_executing() {
        // Even if not all summaries exist, VERIFICATION.md presence means Verified
        let files = vec![
            "01-01-PLAN.md".to_string(),
            "01-01-SUMMARY.md".to_string(),
            "01-VERIFICATION.md".to_string(),
        ];
        let stage = derive_stage(&files, 3);
        assert_eq!(stage, PhaseStage::Verified);
    }

    #[test]
    fn test_derive_stage_unknown_files_only() {
        // Files that don't match any known pattern
        let files = vec!["random-notes.txt".to_string(), "scratch.md".to_string()];
        let stage = derive_stage(&files, 0);
        assert_eq!(stage, PhaseStage::Planned);
    }
}
