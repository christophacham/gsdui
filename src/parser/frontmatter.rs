use serde::de::DeserializeOwned;

use super::ParseError;

/// A parsed document with typed YAML frontmatter metadata and a string body.
#[derive(Debug)]
pub struct Document<T> {
    /// Deserialized frontmatter metadata
    pub metadata: T,
    /// The content after the closing `---` delimiter
    pub content: String,
}

/// Parse a document with YAML frontmatter delimited by `---`.
///
/// Expects the format:
/// ```text
/// ---
/// key: value
/// ---
/// body content here
/// ```
///
/// Handles leading whitespace and UTF-8 BOM before the opening `---`.
pub fn parse_frontmatter<T: DeserializeOwned>(input: &str) -> Result<Document<T>, ParseError> {
    // Strip UTF-8 BOM and leading whitespace
    let trimmed = input.trim_start_matches('\u{FEFF}').trim_start();

    if !trimmed.starts_with("---") {
        return Err(ParseError::NoFrontmatter);
    }

    // Find the closing delimiter after the opening "---"
    let after_open = &trimmed[3..];
    // Skip the rest of the opening line (handle "---\n" or "---\r\n")
    let after_newline = match after_open.find('\n') {
        Some(pos) => &after_open[pos + 1..],
        None => return Err(ParseError::UnclosedFrontmatter),
    };

    // Find the closing "---" on its own line
    let close_pos = find_closing_delimiter(after_newline);
    let close_pos = match close_pos {
        Some(pos) => pos,
        None => return Err(ParseError::UnclosedFrontmatter),
    };

    let yaml_content = &after_newline[..close_pos];
    let body_start = close_pos + 3; // skip "---"
    let body = if body_start < after_newline.len() {
        let rest = &after_newline[body_start..];
        // Skip the newline after closing ---
        if rest.starts_with('\n') {
            &rest[1..]
        } else if rest.starts_with("\r\n") {
            &rest[2..]
        } else {
            rest
        }
    } else {
        ""
    };

    let metadata: T = serde_yml::from_str(yaml_content)?;

    Ok(Document {
        metadata,
        content: body.to_string(),
    })
}

/// Find the position of a closing `---` delimiter that starts at the beginning of a line.
fn find_closing_delimiter(text: &str) -> Option<usize> {
    // Check if text starts with ---
    if text.starts_with("---") && (text.len() == 3 || text[3..].starts_with('\n') || text[3..].starts_with("\r\n") || text[3..].starts_with(' ')) {
        return Some(0);
    }

    // Search for \n--- pattern
    let mut search_from = 0;
    while search_from < text.len() {
        if let Some(pos) = text[search_from..].find("\n---") {
            let abs_pos = search_from + pos + 1; // position of '---' after the newline
            let after = &text[abs_pos + 3..];
            // Verify it's the end of the line (or end of string)
            if after.is_empty() || after.starts_with('\n') || after.starts_with("\r\n") || after.starts_with(' ') {
                return Some(abs_pos);
            }
            search_from = abs_pos + 3;
        } else {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize, PartialEq)]
    struct SimpleMetadata {
        title: String,
        count: i64,
    }

    #[test]
    fn test_parse_valid_frontmatter() {
        let input = "---\ntitle: Hello\ncount: 42\n---\nBody content here.";
        let doc: Document<SimpleMetadata> = parse_frontmatter(input).unwrap();
        assert_eq!(doc.metadata.title, "Hello");
        assert_eq!(doc.metadata.count, 42);
        assert_eq!(doc.content, "Body content here.");
    }

    #[test]
    fn test_parse_frontmatter_with_bom() {
        let input = "\u{FEFF}---\ntitle: BOM Test\ncount: 1\n---\nAfter BOM.";
        let doc: Document<SimpleMetadata> = parse_frontmatter(input).unwrap();
        assert_eq!(doc.metadata.title, "BOM Test");
        assert_eq!(doc.content, "After BOM.");
    }

    #[test]
    fn test_parse_frontmatter_with_leading_whitespace() {
        let input = "  \n  ---\ntitle: Spaced\ncount: 5\n---\nContent.";
        let doc: Document<SimpleMetadata> = parse_frontmatter(input).unwrap();
        assert_eq!(doc.metadata.title, "Spaced");
        assert_eq!(doc.content, "Content.");
    }

    #[test]
    fn test_no_frontmatter() {
        let input = "Just plain text without any frontmatter.";
        let result = parse_frontmatter::<SimpleMetadata>(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::NoFrontmatter => {}
            e => panic!("Expected NoFrontmatter, got: {:?}", e),
        }
    }

    #[test]
    fn test_unclosed_frontmatter() {
        let input = "---\ntitle: Unclosed\ncount: 1\nNo closing delimiter";
        let result = parse_frontmatter::<SimpleMetadata>(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnclosedFrontmatter => {}
            e => panic!("Expected UnclosedFrontmatter, got: {:?}", e),
        }
    }

    #[test]
    fn test_invalid_yaml() {
        let input = "---\ninvalid: [unclosed\n---\n";
        let result = parse_frontmatter::<SimpleMetadata>(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::YamlError(_) => {}
            e => panic!("Expected YamlError, got: {:?}", e),
        }
    }

    #[test]
    fn test_empty_body() {
        let input = "---\ntitle: NoBody\ncount: 0\n---\n";
        let doc: Document<SimpleMetadata> = parse_frontmatter(input).unwrap();
        assert_eq!(doc.metadata.title, "NoBody");
        assert_eq!(doc.content, "");
    }

    #[test]
    fn test_multiline_body() {
        let input = "---\ntitle: Multi\ncount: 2\n---\nLine 1\nLine 2\nLine 3";
        let doc: Document<SimpleMetadata> = parse_frontmatter(input).unwrap();
        assert_eq!(doc.content, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_hashmap_metadata() {
        let input = "---\nfoo: bar\nbaz: 123\n---\n";
        let doc: Document<HashMap<String, serde_json::Value>> = parse_frontmatter(input).unwrap();
        assert_eq!(doc.metadata.get("foo").unwrap(), "bar");
    }
}
