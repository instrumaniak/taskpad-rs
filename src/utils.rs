#![allow(dead_code)]

//! Rust port of `utils.h` / `utils.cpp` from the C++ taskpad codebase.
//!
//! Provides string manipulation, task ID formatting, path normalization,
//! timestamp generation, and T*.md section extraction utilities.

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// to_kebab_case
// ---------------------------------------------------------------------------

/// Convert a string to kebab-case, matching C++ `toKebabCase`.
///
/// Alphanumeric characters are kept; uppercase letters get a `-` prefix
/// (if the result is non-empty and doesn't already end with `-`) then
/// lowered. Spaces, `_`, and `-` become `-`. All other characters are
/// dropped. Trailing `-` is stripped.
pub fn to_kebab_case(name: &str) -> String {
    let mut result = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() {
                if !result.is_empty() && result.as_bytes().last() != Some(&b'-') {
                    result.push('-');
                }
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        } else if (c == ' ' || c == '_' || c == '-')
            && !result.is_empty()
            && result.as_bytes().last() != Some(&b'-')
        {
            result.push('-');
        }
    }
    while result.ends_with('-') {
        result.pop();
    }
    result
}

// ---------------------------------------------------------------------------
// parse_task_id
// ---------------------------------------------------------------------------

/// Parse a task ID string like `"T042"` into its numeric component.
///
/// Returns 0 if the ID is shorter than 4 characters, doesn't start with
/// `'T'`, or has no leading digits after `'T'` (matching C++ `parseTaskId`
/// and `std::stoi` semantics: stops at first non-digit).
pub fn parse_task_id(id: &str) -> i32 {
    if id.len() < 4 || !id.starts_with('T') {
        return 0;
    }
    let digits = &id[1..];
    let mut value: i32 = 0;
    let mut found_digit = false;
    for c in digits.chars() {
        if c.is_ascii_digit() {
            found_digit = true;
            value = value * 10 + (c as i32 - '0' as i32);
        } else {
            break;
        }
    }
    if found_digit { value } else { 0 }
}

// ---------------------------------------------------------------------------
// format_task_id
// ---------------------------------------------------------------------------

/// Format a number as a zero-padded 3-digit task ID string `"TXXX"`.
///
/// Clamps `num` to `[0, 999]` before formatting, matching C++ `formatTaskId`.
pub fn format_task_id(num: i32) -> String {
    let num = num.clamp(0, 999);
    format!("T{:03}", num)
}

// ---------------------------------------------------------------------------
// current_timestamp
// ---------------------------------------------------------------------------

/// Return the current local timestamp as `"YYYY-MM-DD HH:MM"`.
///
/// Uses `date "+%Y-%m-%d %H:%M"` via `std::process::Command` for local
/// time. Falls back to a manual UTC computation from `SystemTime::now()`
/// if the command fails.
pub fn current_timestamp() -> String {
    match Command::new("date").arg("+%Y-%m-%d %H:%M").output() {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .unwrap_or_else(|_| fallback_timestamp())
            .trim()
            .to_string(),
        _ => fallback_timestamp(),
    }
}

/// Manual UTC timestamp computation as fallback (no `chrono` dependency).
fn fallback_timestamp() -> String {
    let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => return "1970-01-01 00:00".to_string(),
    };
    let minutes = secs / 60;
    let hours = minutes / 60;
    let minutes_in_day = hours % 24;
    let days = hours / 24;

    let (year, month, day) = unix_days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        year,
        month,
        day,
        minutes_in_day,
        minutes % 60
    )
}

/// Convert a Unix day count to (year, month, day) using a standard algorithm.
fn unix_days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's date algebra
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = era * 400 + yoe;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

// ---------------------------------------------------------------------------
// split & trim
// ---------------------------------------------------------------------------

/// Split a string by a delimiter, trimming each token (matching C++ `split`).
pub fn split(s: &str, delimiter: char) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    for c in s.chars() {
        if c == delimiter {
            tokens.push(trim(&token));
            token.clear();
        } else {
            token.push(c);
        }
    }
    tokens.push(trim(&token));
    tokens
}

/// Trim whitespace from both ends of a string (matching C++ `trim`).
pub fn trim(s: &str) -> String {
    let start = s.find(|c: char| !c.is_whitespace()).unwrap_or(s.len());
    let end = s
        .rfind(|c: char| !c.is_whitespace())
        .map(|e| e + 1)
        .unwrap_or(0);
    if start >= end {
        String::new()
    } else {
        s[start..end].to_string()
    }
}

// ---------------------------------------------------------------------------
// normalize_path
// ---------------------------------------------------------------------------

/// Normalize a path: replace backslashes with forward slashes and strip
/// trailing slashes (keeping a single `/` if the path is just `/`).
pub fn normalize_path(path: &str) -> String {
    let mut result = path.replace('\\', "/");
    while result.len() > 1 && result.ends_with('/') {
        result.pop();
    }
    result
}

// ---------------------------------------------------------------------------
// resolve_task_dir
// ---------------------------------------------------------------------------

/// Resolve the task directory: return `tasks_dir` if non-empty, otherwise
/// read from the project config; fall back to `"specs/tasks"` on error.
pub fn resolve_task_dir(tasks_dir: &str) -> String {
    if !tasks_dir.is_empty() {
        return tasks_dir.to_string();
    }
    match crate::storage::read_task_dir(".") {
        Ok(dir) => dir,
        Err(_) => "specs/tasks".to_string(),
    }
}

// ---------------------------------------------------------------------------
// extract_phase
// ---------------------------------------------------------------------------

/// Extract the phase number from a T*.md content string.
///
/// Searches for `"## Phase:"`, reads the integer that follows (after
/// whitespace), and clamps negative values to 0. Returns 0 if not found
/// or if no digits follow the colon.
pub fn extract_phase(content: &str) -> i32 {
    let pos = match content.find("## Phase:") {
        Some(p) => p,
        None => return 0,
    };
    let colon_offset = match content[pos..].find(':') {
        Some(p) => p,
        None => return 0,
    };
    let mut value_start = pos + colon_offset + 1;
    while value_start < content.len() && content[value_start..].starts_with(char::is_whitespace) {
        value_start += 1;
    }
    let mut value_end = value_start;
    while value_end < content.len()
        && content[value_end..].starts_with(|c: char| c.is_ascii_digit())
    {
        value_end += 1;
    }
    if value_end == value_start {
        return 0;
    }
    let phase: i32 = content[value_start..value_end].parse().unwrap_or(0);
    if phase < 0 { 0 } else { phase }
}

// ---------------------------------------------------------------------------
// extract_critical
// ---------------------------------------------------------------------------

/// Extract the critical flag from a T*.md content string.
///
/// Searches for `"## Critical:"`, reads the token after the colon (until
/// whitespace or newline), lowercases it, and returns `true` only if it
/// equals `"true"`.
pub fn extract_critical(content: &str) -> bool {
    let pos = match content.find("## Critical:") {
        Some(p) => p,
        None => return false,
    };
    let colon_offset = match content[pos..].find(':') {
        Some(p) => p,
        None => return false,
    };
    let mut value_start = pos + colon_offset + 1;
    while value_start < content.len() && content[value_start..].starts_with(char::is_whitespace) {
        value_start += 1;
    }
    let mut value_end = value_start;
    while value_end < content.len()
        && !content[value_end..].starts_with(char::is_whitespace)
        && !content[value_end..].starts_with('\n')
        && !content[value_end..].starts_with('\r')
    {
        value_end += 1;
    }
    let value = &content[value_start..value_end];
    value.eq_ignore_ascii_case("true")
}

// ---------------------------------------------------------------------------
// extract_section_list_items
// ---------------------------------------------------------------------------

/// Extract backtick-delimited items from a section of T*.md content.
///
/// Finds `section_header` in `content`, then extracts the text between
/// the newline after the header and the next `"\\n## "` marker (or EOF).
/// Within that section, returns the text inside backtick pairs on each
/// line.
pub fn extract_section_list_items(content: &str, section_header: &str) -> Vec<String> {
    let mut result = Vec::new();

    let section_pos = match content.find(section_header) {
        Some(p) => p,
        None => return result,
    };
    let after_header = &content[section_pos + section_header.len()..];
    let content_start = match after_header.find('\n') {
        Some(p) => section_pos + section_header.len() + p + 1,
        None => return result,
    };
    let remainder = &content[content_start..];
    let section_end = content_start + remainder.find("\n## ").unwrap_or(remainder.len());

    let section = &content[content_start..section_end];

    let mut line_start = 0;
    while line_start < section.len() {
        let line_end = section[line_start..]
            .find('\n')
            .map(|p| line_start + p)
            .unwrap_or(section.len());
        let line = &section[line_start..line_end];

        if let Some(tick_start) = line.find('`') {
            let after_tick = &line[tick_start + 1..];
            if let Some(tick_end) = after_tick.find('`') {
                result.push(line[tick_start + 1..tick_start + 1 + tick_end].to_string());
            }
        }

        line_start = line_end + 1;
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("Project Setup"), "project-setup");
        assert_eq!(to_kebab_case("CoreTypes"), "core-types");
        assert_eq!(to_kebab_case("simple"), "simple");
        assert_eq!(to_kebab_case(""), "");
        assert_eq!(to_kebab_case("  spaces  "), "spaces");
        assert_eq!(to_kebab_case("Already-Kebab"), "already-kebab");
        assert_eq!(to_kebab_case("snake_case"), "snake-case");
    }

    #[test]
    fn test_parse_task_id() {
        assert_eq!(parse_task_id("T001"), 1);
        assert_eq!(parse_task_id("T042"), 42);
        assert_eq!(parse_task_id("T999"), 999);
        assert_eq!(parse_task_id("T000"), 0);
    }

    #[test]
    fn test_format_task_id() {
        assert_eq!(format_task_id(1), "T001");
        assert_eq!(format_task_id(42), "T042");
        assert_eq!(format_task_id(999), "T999");
        assert_eq!(format_task_id(0), "T000");
        assert_eq!(format_task_id(1000), "T999");
        assert_eq!(format_task_id(-1), "T000");
    }

    #[test]
    fn test_current_timestamp_format() {
        let ts = current_timestamp();
        assert_eq!(ts.len(), 16);
        assert_eq!(ts.chars().nth(4), Some('-'));
        assert_eq!(ts.chars().nth(7), Some('-'));
        assert_eq!(ts.chars().nth(10), Some(' '));
        assert_eq!(ts.chars().nth(13), Some(':'));
    }

    #[test]
    fn test_split() {
        let parts = split("a,b,c", ',');
        assert_eq!(parts, vec!["a", "b", "c"]);
        let parts = split("single", ',');
        assert_eq!(parts, vec!["single"]);
        let parts = split("", ',');
        assert_eq!(parts, vec![""]);
    }

    #[test]
    fn test_trim() {
        assert_eq!(trim("  hello  "), "hello");
        assert_eq!(trim("  "), "");
        assert_eq!(trim(""), "");
        assert_eq!(trim("a b"), "a b");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("specs/tasks/"), "specs/tasks");
        assert_eq!(normalize_path("specs/tasks"), "specs/tasks");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_extract_phase() {
        assert_eq!(extract_phase("## Phase: 3"), 3);
        assert_eq!(extract_phase("## Phase: 0"), 0);
        assert_eq!(extract_phase("## Phase: 42"), 42);
        assert_eq!(extract_phase("no phase header here"), 0);
        assert_eq!(extract_phase(""), 0);
        assert_eq!(extract_phase("## Phase: -1"), 0);
        assert_eq!(extract_phase("## Phase: abc"), 0);
        assert_eq!(extract_phase("## Phase: 7\n## Goal: stuff"), 7);
        assert_eq!(extract_phase("## Phase:"), 0);
    }

    #[test]
    fn test_extract_critical() {
        assert!(extract_critical("## Critical: true"));
        assert!(extract_critical("## Critical: TRUE"));
        assert!(extract_critical("## Critical: True"));
        assert!(extract_critical("## Critical: tRuE"));
        assert!(!extract_critical("## Critical: false"));
        assert!(!extract_critical("## Critical: FALSE"));
        assert!(!extract_critical("no critical header"));
        assert!(!extract_critical(""));
        assert!(!extract_critical("## Critical: garbage"));
        assert!(!extract_critical("## Critical: maybe"));
        assert!(extract_critical("## Critical: true\n## Goal: stuff"));
    }

    #[test]
    fn test_extract_section_list_items() {
        let content = "# T001: Test\n## Spec References\n- `spec1.md`\n- `path/to/spec2.md`\n## Files to Create/Modify\n- `src/file1.cpp`\n- `include/file2.h`\n".to_string();

        let specs = extract_section_list_items(&content, "## Spec References");
        assert_eq!(specs, vec!["spec1.md", "path/to/spec2.md"]);

        let files = extract_section_list_items(&content, "## Files to Create/Modify");
        assert_eq!(files, vec!["src/file1.cpp", "include/file2.h"]);

        let missing = extract_section_list_items(&content, "## Nonexistent Section");
        assert!(missing.is_empty());

        let empty = extract_section_list_items("", "## Spec References");
        assert!(empty.is_empty());

        let no_ticks = extract_section_list_items(
            "## Spec References\n- plain text\n- more text\n",
            "## Spec References",
        );
        assert!(no_ticks.is_empty());

        let mixed = "## Files\n- `main.cpp` (MODIFY)\n- Makefile\n- `utils.h`\n".to_string();
        let mixed_result = extract_section_list_items(&mixed, "## Files");
        assert_eq!(mixed_result, vec!["main.cpp", "utils.h"]);
    }
}
