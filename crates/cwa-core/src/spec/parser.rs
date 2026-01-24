//! Parser for splitting long prompts into multiple spec entries.

/// A parsed spec entry extracted from a long prompt.
#[derive(Debug, Clone)]
pub struct ParsedSpec {
    pub title: String,
    pub description: Option<String>,
    pub priority: String,
}

/// Parse a long prompt into multiple spec entries.
///
/// Supports the following formats:
/// - Numbered lists: `1. Title\n   Description`
/// - Bullet points: `- Title` or `* Title`
/// - Markdown headings: `# Title` or `## Title`
/// - Paragraph-separated blocks (fallback)
pub fn parse_prompt(input: &str) -> Vec<ParsedSpec> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // Try each parser in order of specificity
    let results = try_numbered_list(trimmed);
    if results.len() > 1 {
        return results;
    }

    let results = try_bullet_list(trimmed);
    if results.len() > 1 {
        return results;
    }

    let results = try_headings(trimmed);
    if results.len() > 1 {
        return results;
    }

    let results = try_paragraphs(trimmed);
    if results.len() > 1 {
        return results;
    }

    // Fallback: single spec with the whole text
    vec![make_spec_from_block(trimmed)]
}

/// Try parsing as a numbered list (1. item, 2. item, etc.)
fn try_numbered_list(input: &str) -> Vec<ParsedSpec> {
    let mut specs = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();

    for line in input.lines() {
        let stripped = line.trim_start();
        if is_numbered_item(stripped) {
            if !current_lines.is_empty() {
                specs.push(make_spec_from_numbered(&current_lines));
            }
            current_lines = vec![stripped];
        } else if !current_lines.is_empty() {
            current_lines.push(line);
        } else if !line.trim().is_empty() {
            // Non-numbered content before the first item — not a numbered list
            return Vec::new();
        }
    }

    if !current_lines.is_empty() {
        specs.push(make_spec_from_numbered(&current_lines));
    }

    specs
}

/// Try parsing as bullet list (- item or * item)
fn try_bullet_list(input: &str) -> Vec<ParsedSpec> {
    let mut specs = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();

    for line in input.lines() {
        let stripped = line.trim_start();
        if is_bullet_item(stripped) {
            if !current_lines.is_empty() {
                specs.push(make_spec_from_bullet(&current_lines));
            }
            current_lines = vec![stripped];
        } else if !current_lines.is_empty() {
            current_lines.push(line);
        } else if !line.trim().is_empty() {
            return Vec::new();
        }
    }

    if !current_lines.is_empty() {
        specs.push(make_spec_from_bullet(&current_lines));
    }

    specs
}

/// Try parsing by markdown headings (# or ##)
fn try_headings(input: &str) -> Vec<ParsedSpec> {
    let mut specs = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_body: Vec<&str> = Vec::new();

    for line in input.lines() {
        let stripped = line.trim_start();
        if stripped.starts_with('#') {
            if let Some(title) = current_title.take() {
                specs.push(make_spec_from_title_body(&title, &current_body));
                current_body.clear();
            }
            let title = stripped.trim_start_matches('#').trim().to_string();
            if !title.is_empty() {
                current_title = Some(title);
            }
        } else if current_title.is_some() {
            current_body.push(line);
        }
    }

    if let Some(title) = current_title {
        specs.push(make_spec_from_title_body(&title, &current_body));
    }

    specs
}

/// Try splitting by double newlines (paragraphs)
fn try_paragraphs(input: &str) -> Vec<ParsedSpec> {
    input
        .split("\n\n")
        .map(|block| block.trim())
        .filter(|block| !block.is_empty())
        .map(|block| make_spec_from_block(block))
        .collect()
}

fn is_numbered_item(line: &str) -> bool {
    let mut chars = line.chars();
    // Must start with one or more digits
    let first = chars.next();
    if !first.map_or(false, |c| c.is_ascii_digit()) {
        return false;
    }
    for c in chars.by_ref() {
        if c == '.' || c == ')' {
            // Must be followed by a space
            return chars.next().map_or(false, |c| c == ' ');
        }
        if !c.is_ascii_digit() {
            return false;
        }
    }
    false
}

fn is_bullet_item(line: &str) -> bool {
    (line.starts_with("- ") || line.starts_with("* ")) && line.len() > 2
}

fn make_spec_from_numbered(lines: &[&str]) -> ParsedSpec {
    let first = lines[0];
    // Strip "N. " or "N) " prefix
    let title_start = first.find(|c: char| c == '.' || c == ')').unwrap_or(0) + 1;
    let title = first[title_start..].trim();

    let (title, description) = extract_title_and_desc(title, &lines[1..]);

    ParsedSpec {
        title,
        description,
        priority: "medium".to_string(),
    }
}

fn make_spec_from_bullet(lines: &[&str]) -> ParsedSpec {
    let first = lines[0];
    // Strip "- " or "* " prefix
    let title = &first[2..];

    let (title, description) = extract_title_and_desc(title, &lines[1..]);

    ParsedSpec {
        title,
        description,
        priority: "medium".to_string(),
    }
}

fn make_spec_from_title_body(title: &str, body: &[&str]) -> ParsedSpec {
    let desc = body
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    ParsedSpec {
        title: truncate_title(title),
        description: if desc.is_empty() { None } else { Some(desc) },
        priority: "medium".to_string(),
    }
}

fn make_spec_from_block(block: &str) -> ParsedSpec {
    let lines: Vec<&str> = block.lines().collect();
    let first_line = lines.first().map(|s| s.trim()).unwrap_or("");

    let (title, description) = extract_title_and_desc(first_line, &lines[1..]);

    ParsedSpec {
        title,
        description,
        priority: "medium".to_string(),
    }
}

/// Extract title (first line, possibly truncated) and description (remaining lines).
fn extract_title_and_desc(first_line: &str, rest: &[&str]) -> (String, Option<String>) {
    let title = truncate_title(first_line);

    let desc_parts: Vec<&str> = rest.iter().map(|s| s.trim()).collect();
    let desc = desc_parts.join("\n").trim().to_string();

    let description = if desc.is_empty() {
        // If title was truncated, put the full text as description
        if first_line.len() > 120 {
            Some(first_line.to_string())
        } else {
            None
        }
    } else {
        // Combine full first line (if truncated) with rest
        if first_line.len() > 120 {
            Some(format!("{}\n{}", first_line, desc))
        } else {
            Some(desc)
        }
    };

    (title, description)
}

/// Truncate a title to a reasonable length for display.
fn truncate_title(s: &str) -> String {
    let s = s.trim();
    if s.len() <= 120 {
        s.to_string()
    } else {
        // Find a word boundary near 120 chars
        let boundary = s[..120]
            .rfind(' ')
            .unwrap_or(120);
        format!("{}...", &s[..boundary])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numbered_list() {
        let input = "1. User Authentication\n   JWT-based login system\n2. Dashboard\n   Main overview page\n3. Settings Page";
        let specs = parse_prompt(input);
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].title, "User Authentication");
        assert_eq!(specs[0].description, Some("JWT-based login system".to_string()));
        assert_eq!(specs[1].title, "Dashboard");
        assert_eq!(specs[2].title, "Settings Page");
    }

    #[test]
    fn test_bullet_list() {
        let input = "- User login with OAuth\n- Profile management\n- Notification system";
        let specs = parse_prompt(input);
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].title, "User login with OAuth");
        assert_eq!(specs[1].title, "Profile management");
        assert_eq!(specs[2].title, "Notification system");
    }

    #[test]
    fn test_headings() {
        let input = "# Authentication\nHandle user login\n\n# Authorization\nRole-based access";
        let specs = parse_prompt(input);
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].title, "Authentication");
        assert_eq!(specs[0].description, Some("Handle user login".to_string()));
    }

    #[test]
    fn test_paragraphs() {
        let input = "Build a user registration form with email validation\n\nCreate an admin panel for user management\n\nAdd export functionality for reports";
        let specs = parse_prompt(input);
        assert_eq!(specs.len(), 3);
    }

    #[test]
    fn test_single_item() {
        let input = "Build a complete user authentication system";
        let specs = parse_prompt(input);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].title, "Build a complete user authentication system");
    }

    #[test]
    fn test_empty_input() {
        let specs = parse_prompt("");
        assert_eq!(specs.len(), 0);
    }

    #[test]
    fn test_long_title_truncation() {
        let long = "A".repeat(200);
        let input = format!("- {}\n- Short item", long);
        let specs = parse_prompt(&input);
        assert_eq!(specs.len(), 2);
        assert!(specs[0].title.len() <= 123); // 120 + "..."
        assert!(specs[0].title.ends_with("..."));
    }
}
