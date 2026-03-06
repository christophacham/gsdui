pub mod frontmatter;
pub mod state_md;
pub mod roadmap;
pub mod plan;
pub mod summary;

use std::fmt;

/// Unified error type for all GSD file parsers.
#[derive(Debug)]
pub enum ParseError {
    /// Input contains no YAML frontmatter delimiters
    NoFrontmatter,
    /// Opening `---` found but no closing `---`
    UnclosedFrontmatter,
    /// YAML content between delimiters is invalid
    YamlError(serde_yml::Error),
    /// JSON content is invalid
    JsonError(serde_json::Error),
    /// Regex pattern failed to compile or match
    RegexError(String),
    /// I/O error reading file content
    IoError(std::io::Error),
    /// Content structure is invalid (e.g., missing required sections)
    InvalidContent(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::NoFrontmatter => write!(f, "No YAML frontmatter found"),
            ParseError::UnclosedFrontmatter => {
                write!(f, "Unclosed YAML frontmatter (missing closing ---)")
            }
            ParseError::YamlError(e) => write!(f, "YAML parse error: {}", e),
            ParseError::JsonError(e) => write!(f, "JSON parse error: {}", e),
            ParseError::RegexError(e) => write!(f, "Regex error: {}", e),
            ParseError::IoError(e) => write!(f, "I/O error: {}", e),
            ParseError::InvalidContent(msg) => write!(f, "Invalid content: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::YamlError(e) => Some(e),
            ParseError::JsonError(e) => Some(e),
            ParseError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_yml::Error> for ParseError {
    fn from(e: serde_yml::Error) -> Self {
        ParseError::YamlError(e)
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(e: serde_json::Error) -> Self {
        ParseError::JsonError(e)
    }
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::IoError(e)
    }
}
