use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// --- Enums ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Offline,
}

impl fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectStatus::Active => write!(f, "active"),
            ProjectStatus::Offline => write!(f, "offline"),
        }
    }
}

impl FromStr for ProjectStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(ProjectStatus::Active),
            "offline" => Ok(ProjectStatus::Offline),
            other => Err(format!("Unknown project status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStage {
    Planned,
    Discussed,
    Researched,
    PlannedReady,
    Executing,
    Executed,
    Verified,
}

impl fmt::Display for PhaseStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseStage::Planned => write!(f, "planned"),
            PhaseStage::Discussed => write!(f, "discussed"),
            PhaseStage::Researched => write!(f, "researched"),
            PhaseStage::PlannedReady => write!(f, "planned_ready"),
            PhaseStage::Executing => write!(f, "executing"),
            PhaseStage::Executed => write!(f, "executed"),
            PhaseStage::Verified => write!(f, "verified"),
        }
    }
}

impl FromStr for PhaseStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "planned" => Ok(PhaseStage::Planned),
            "discussed" => Ok(PhaseStage::Discussed),
            "researched" => Ok(PhaseStage::Researched),
            "planned_ready" => Ok(PhaseStage::PlannedReady),
            "executing" => Ok(PhaseStage::Executing),
            "executed" => Ok(PhaseStage::Executed),
            "verified" => Ok(PhaseStage::Verified),
            other => Err(format!("Unknown phase stage: {}", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Pending,
    Working,
    Done,
    Failed,
}

impl fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanStatus::Pending => write!(f, "pending"),
            PlanStatus::Working => write!(f, "working"),
            PlanStatus::Done => write!(f, "done"),
            PlanStatus::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for PlanStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(PlanStatus::Pending),
            "working" => Ok(PlanStatus::Working),
            "done" => Ok(PlanStatus::Done),
            "failed" => Ok(PlanStatus::Failed),
            other => Err(format!("Unknown plan status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    GapsFound,
    HumanNeeded,
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationStatus::Passed => write!(f, "passed"),
            VerificationStatus::GapsFound => write!(f, "gaps_found"),
            VerificationStatus::HumanNeeded => write!(f, "human_needed"),
        }
    }
}

impl FromStr for VerificationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "passed" => Ok(VerificationStatus::Passed),
            "gaps_found" => Ok(VerificationStatus::GapsFound),
            "human_needed" => Ok(VerificationStatus::HumanNeeded),
            other => Err(format!("Unknown verification status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParseErrorSeverity {
    Warning,
    Error,
}

impl fmt::Display for ParseErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorSeverity::Warning => write!(f, "warning"),
            ParseErrorSeverity::Error => write!(f, "error"),
        }
    }
}

impl FromStr for ParseErrorSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "warning" => Ok(ParseErrorSeverity::Warning),
            "error" => Ok(ParseErrorSeverity::Error),
            other => Err(format!("Unknown parse error severity: {}", other)),
        }
    }
}

// --- Row structs ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub status: String,
    pub retention_days: Option<i64>,
    pub created_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PhaseState {
    pub id: i64,
    pub project_id: String,
    pub phase_number: String,
    pub phase_name: String,
    pub goal: Option<String>,
    pub depends_on: Option<String>,
    pub stage: String,
    pub status: Option<String>,
    pub requirements: Option<String>,
    pub plan_count: Option<i64>,
    pub completed_plan_count: Option<i64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlanState {
    pub id: i64,
    pub project_id: String,
    pub phase_number: String,
    pub plan_number: String,
    pub plan_name: Option<String>,
    pub wave: Option<i64>,
    pub depends_on: Option<String>,
    pub plan_type: Option<String>,
    pub status: String,
    pub requirements: Option<String>,
    pub files_modified: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionRun {
    pub id: i64,
    pub project_id: String,
    pub phase_number: String,
    pub plan_number: String,
    pub run_number: i64,
    pub superseded: i64,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_minutes: Option<f64>,
    pub status: Option<String>,
    pub key_files_created: Option<String>,
    pub key_files_modified: Option<String>,
    pub requirements_completed: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Commit {
    pub id: i64,
    pub execution_run_id: i64,
    pub task_number: i64,
    pub task_name: Option<String>,
    pub commit_hash: Option<String>,
    pub commit_type: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentSession {
    pub id: i64,
    pub project_id: String,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub phase_number: Option<String>,
    pub plan_number: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VerificationResult {
    pub id: i64,
    pub project_id: String,
    pub phase_number: String,
    pub status: String,
    pub score: Option<String>,
    pub verified_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ParseError {
    pub id: i64,
    pub project_id: String,
    pub file_path: String,
    pub error_message: String,
    pub severity: String,
    pub occurred_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectConfig {
    pub id: i64,
    pub project_id: String,
    pub config_json: String,
    pub updated_at: String,
}
