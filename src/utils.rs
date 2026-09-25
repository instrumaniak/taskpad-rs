//! Rust port of `utils.h` / `utils.cpp` from the C++ taskpad codebase.
//!
//! Provides string manipulation, task ID formatting, path normalization,
//! timestamp generation, and T*.md section extraction utilities.
//!
//! This module is deliberately leaf-level: it must not depend on `storage`
//! (or any other crate module). Project-dir resolution lives in
//! `storage::resolve_task_dir`, next to `storage::read_task_dir`.

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
pub(crate) fn to_kebab_case(name: &str) -> String {
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
///
/// Overflow handling intentionally differs from C++: `std::stoi` throws
/// `std::out_of_range` on overflow (e.g. a hand-written `T9999999999`
/// key), but this signature has no error channel and callers
/// (`find_next_task_id`, `status`, `next`) use the value only for
/// ordering. The accumulator is `i64` with `checked_mul`/`checked_add`
/// and saturates at `i32::MAX`, so huge IDs sort after every real ID
/// instead of wrapping (in release builds plain `i32` arithmetic would
/// wrap, e.g. `T9999999999` wrapped to `1410065407`).
pub(crate) fn parse_task_id(id: &str) -> i32 {
    if id.len() < 4 || !id.starts_with('T') {
        return 0;
    }
    let digits = &id[1..];
    let mut value: i64 = 0;
    let mut found_digit = false;
    for c in digits.chars() {
        if c.is_ascii_digit() {
            found_digit = true;
            let digit = (c as i64) - ('0' as i64);
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => return i32::MAX,
            }
            if value > i32::MAX as i64 {
                return i32::MAX;
            }
        } else {
            break;
        }
    }
    if found_digit { value as i32 } else { 0 }
}

// ---------------------------------------------------------------------------
// format_task_id
// ---------------------------------------------------------------------------

/// Format a number as a zero-padded 3-digit task ID string `"TXXX"`.
///
/// Clamps `num` to `[0, 999]` before formatting, matching C++ `formatTaskId`.
pub(crate) fn format_task_id(num: i32) -> String {
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
pub(crate) fn current_timestamp() -> String {
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
pub(crate) fn split(s: &str, delimiter: char) -> Vec<String> {
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
///
/// Walks char boundaries rather than raw byte offsets: `rfind` returns the
/// *start* offset of the last non-whitespace character, which is only the
/// matching *end* offset for a one-byte character, so `offset + 1` used to
/// land inside a trailing multibyte character and panic. Because
/// `char_indices().rev().next()` yields the whole character, `offset +
/// c.len_utf8()` is the end offset for any character.
///
/// Whitespace is `char::is_whitespace`, i.e. Unicode-aware, so a non-ASCII
/// space such as U+00A0 is trimmed. C++'s `std::isspace` in the C locale
/// would not trim it; that difference is pre-existing and kept as is,
/// because changing it would alter the printed bytes of every command that
/// echoes T*.md text.
pub(crate) fn trim(s: &str) -> String {
    let start = match s.find(|c: char| !c.is_whitespace()) {
        Some(start) => start,
        None => return String::new(),
    };
    let end = match s.char_indices().rev().find(|(_, c)| !c.is_whitespace()) {
        Some((offset, c)) => offset + c.len_utf8(),
        None => return String::new(),
    };
    match s.get(start..end) {
        Some(trimmed) => trimmed.to_string(),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// normalize_path
// ---------------------------------------------------------------------------

/// Normalize a path: replace backslashes with forward slashes and strip
/// trailing slashes (keeping a single `/` if the path is just `/`).
pub(crate) fn normalize_path(path: &str) -> String {
    let mut result = path.replace('\\', "/");
    while result.len() > 1 && result.ends_with('/') {
        result.pop();
    }
    result
}

// ---------------------------------------------------------------------------
// extract_phase
// ---------------------------------------------------------------------------

/// Walk `bytes` one byte at a time from `from` for as long as `predicate`
/// holds, returning the offset where it stopped.
///
/// This is the direct equivalent of the C++ scanner loops
/// (`utils.cpp:108-115`, `utils.cpp:131-139`), which step one `char` at a
/// time over `std::isspace` / `std::isdigit`. In the C locale those
/// classify ASCII only, so a multibyte character never matches — and
/// walking `as_bytes()` reproduces that without ever having to slice a
/// `str`: every byte checked is a raw byte, never a `char` index.
///
/// The one thing this relies on is that a run started at a char boundary
/// can only *stop* at one: a UTF-8 continuation byte (0x80-0xBF) is never
/// ASCII whitespace or an ASCII digit, so neither `!is_ascii_whitespace`
/// nor `is_ascii_digit` can be satisfied by one.
fn byte_offset_while<F: Fn(u8) -> bool>(bytes: &[u8], from: usize, predicate: F) -> usize {
    let mut offset = from;
    while bytes.get(offset).is_some_and(|b| predicate(*b)) {
        offset += 1;
    }
    offset
}

/// Extract the phase number from a T*.md content string.
///
/// Searches for `"## Phase:"`, reads the integer that follows (after
/// whitespace), and clamps negative values to 0. Returns 0 if not found
/// or if no digits follow the colon.
///
/// Only ASCII whitespace is skipped and only ASCII digits are consumed, so
/// a multibyte character between the colon and the number (an NBSP, an
/// emoji) ends the scan exactly as the C++ byte scanner ends it — yielding
/// 0 rather than panicking on a mid-codepoint slice.
pub(crate) fn extract_phase(content: &str) -> i32 {
    let pos = match content.find("## Phase:") {
        Some(p) => p,
        None => return 0,
    };
    let colon_offset = match content[pos..].find(':') {
        Some(p) => p,
        None => return 0,
    };
    let bytes = content.as_bytes();
    let value_start = byte_offset_while(bytes, pos + colon_offset + 1, |b| b.is_ascii_whitespace());
    let value_end = byte_offset_while(bytes, value_start, |b| b.is_ascii_digit());
    if value_end == value_start {
        return 0;
    }
    match content.get(value_start..value_end) {
        // `str::get` cannot fail for offsets produced by
        // `byte_offset_while`; the `None` arm only exists so no code path
        // can panic on a hand-built offset.
        Some(digits) => {
            let phase: i32 = digits.parse().unwrap_or(0);
            if phase < 0 { 0 } else { phase }
        }
        None => 0,
    }
}

// ---------------------------------------------------------------------------
// extract_critical
// ---------------------------------------------------------------------------

/// Extract the critical flag from a T*.md content string.
///
/// Searches for `"## Critical:"`, reads the token after the colon (until
/// whitespace or newline), lowercases it, and returns `true` only if it
/// equals `"true"`.
///
/// The token is delimited by ASCII whitespace, so a multibyte character
/// (NBSP, `→`, an emoji) inside the token is consumed byte-for-byte and the
/// token simply fails the `"true"` comparison — the same result the C++
/// byte scanner gives, instead of a panic on a mid-codepoint slice.
pub(crate) fn extract_critical(content: &str) -> bool {
    let pos = match content.find("## Critical:") {
        Some(p) => p,
        None => return false,
    };
    let colon_offset = match content[pos..].find(':') {
        Some(p) => p,
        None => return false,
    };
    let bytes = content.as_bytes();
    let value_start = byte_offset_while(bytes, pos + colon_offset + 1, |b| b.is_ascii_whitespace());
    // C++ stops at any `std::isspace` byte, which already covers '\n' and
    // '\r' (both are ASCII whitespace), so the separate checks it spells out
    // need no counterpart here.
    let value_end = byte_offset_while(bytes, value_start, |b| !b.is_ascii_whitespace());
    match content.get(value_start..value_end) {
        Some(value) => value.eq_ignore_ascii_case("true"),
        None => false,
    }
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
///
/// Only the first backtick pair on a line counts and an unclosed backtick
/// is ignored, matching C++ `utils.cpp:175-181`. Locating the ticks with
/// `split_once` instead of raw byte offsets keeps the extracted item on a
/// char boundary, so a multibyte character inside a backtick pair (or
/// anywhere else in the section) is copied out intact.
pub(crate) fn extract_section_list_items(content: &str, section_header: &str) -> Vec<String> {
    let mut result = Vec::new();

    let section_pos = match content.find(section_header) {
        Some(p) => p,
        None => return result,
    };
    let after_header = match content.get(section_pos + section_header.len()..) {
        Some(rest) => rest,
        None => return result,
    };
    let content_start = match after_header.find('\n') {
        Some(p) => section_pos + section_header.len() + p + 1,
        None => return result,
    };
    let section = match content.get(content_start..) {
        Some(remainder) => match remainder.find("\n## ") {
            Some(p) => &remainder[..p],
            None => remainder,
        },
        None => return result,
    };

    // C++ walks the section line by line by hand. `split('\n')` yields the
    // same lines, plus a harmless empty tail when the section ends in a
    // newline (an empty line has no backticks, so it contributes nothing).
    for line in section.split('\n') {
        if let Some((_, after_tick)) = line.split_once('`')
            && let Some((item, _)) = after_tick.split_once('`')
        {
            result.push(item.to_string());
        }
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
        // Stop-at-first-non-digit (`std::stoi`) semantics.
        assert_eq!(parse_task_id("T12x"), 12);
        assert_eq!(parse_task_id("T001extra"), 1);
        assert_eq!(parse_task_id("T12"), 0);
    }

    #[test]
    fn test_parse_task_id_overflow_saturates() {
        // `T9999999999` overflows `i32` (plain `i32` arithmetic wrapped it
        // to `1410065407` in release builds); C++ `std::stoi` would throw
        // `std::out_of_range` here. Rust saturates at `i32::MAX` so the
        // huge ID still sorts after every real ID.
        assert_eq!(parse_task_id("T9999999999"), i32::MAX);
        assert!(parse_task_id("T9999999999") > parse_task_id("T999"));
        assert_eq!(parse_task_id("T2147483648"), i32::MAX);
        assert_eq!(parse_task_id("T2147483647"), i32::MAX);
        // Even an `i64`-overflowing digit run saturates instead of wrapping.
        assert_eq!(parse_task_id("T99999999999999999999999"), i32::MAX);
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
    fn test_trim_multibyte_at_both_ends() {
        // A trailing multibyte character used to make `rfind(..) + 1` land
        // inside it and panic.
        assert_eq!(trim("  é  "), "é");
        assert_eq!(trim("  →  "), "→");
        assert_eq!(trim("  \u{1f600}  "), "\u{1f600}");
        assert_eq!(trim("é \u{1f600}"), "é \u{1f600}");
        assert_eq!(trim(" é "), "é");
        assert_eq!(trim("\u{1f600}"), "\u{1f600}");
        // U+00A0 is whitespace to `char::is_whitespace`, so a string made of
        // nothing but NBSPs trims to empty (pre-existing Unicode-aware
        // behaviour; C++'s C-locale `isspace` would keep the bytes).
        assert_eq!(trim("  \u{a0}  "), "");
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

    // ---- multibyte content must not panic, and must match the C++ byte scan ----
    //
    // The C++ scanners step one `char` at a time over `std::isspace` /
    // `std::isdigit`, which in the C locale classify ASCII only. A Rust port
    // that advanced by one *byte* and then indexed a `str` would panic on a
    // mid-codepoint boundary; one that used `char::is_whitespace` would
    // diverge from C++ (U+00A0 is whitespace to Rust, not to `isspace`).
    // Both are pinned here.

    /// A 2-byte, a 3-byte and a 4-byte character, plus an ASCII arrow.
    const MULTIBYTE: &str = "é \u{2192} \u{a0} \u{1f600}";

    #[test]
    fn test_extract_phase_multibyte_around_the_value() {
        // Multibyte characters before the number: `isspace` does not match
        // them in the C locale, so the scan stops with no digits => 0.
        assert_eq!(extract_phase("## Phase: \u{a0} 3"), 0);
        assert_eq!(extract_phase(&format!("## Phase: {MULTIBYTE} 3")), 0);
        // Multibyte characters after the number are irrelevant: the digits
        // run is found first.
        assert_eq!(extract_phase(&format!("## Phase: 3 {MULTIBYTE}")), 3);
        assert_eq!(
            extract_phase(&format!("## Phase: 42{MULTIBYTE}\n## Goal: x")),
            42
        );
        // Still no panic on any of the above (the old byte-stepping version
        // panicked on the NBSP case).
        assert!(
            catch_unwind(|| extract_phase("## Phase: \u{a0}\u{1f600} 1")),
            "extract_phase must not panic on multibyte input"
        );
    }

    #[test]
    fn test_extract_critical_multibyte_around_the_value() {
        // A multibyte character glued to "true" makes the token something
        // else, exactly as the C++ byte comparison does.
        assert!(!extract_critical("## Critical: \u{a0}true"));
        assert!(!extract_critical(&format!("## Critical: {MULTIBYTE}true")));
        // Whitespace still terminates the token, so a trailing multibyte
        // character does not break a valid flag.
        assert!(extract_critical(&format!("## Critical: true {MULTIBYTE}")));
        assert!(!extract_critical(&format!("## Critical: {MULTIBYTE}")));
        assert!(
            catch_unwind(|| extract_critical("## Critical: \u{a0}\u{1f600}true")),
            "extract_critical must not panic on multibyte input"
        );
    }

    #[test]
    fn test_extract_section_list_items_multibyte() {
        // Items keep their multibyte characters, in order, and an unclosed
        // backtick on a line is still ignored.
        let content = format!(
            "## Files to Create/Modify\n- `{MULTIBYTE}.rs`\n- plain\n- `unclosed\n- `second.md`\n## Spec References\n"
        );
        let files = extract_section_list_items(&content, "## Files to Create/Modify");
        assert_eq!(
            files,
            vec![format!("{MULTIBYTE}.rs"), "second.md".to_string()]
        );
        assert!(extract_section_list_items(&content, "## Spec References").is_empty());
    }

    /// Run `f` and return `true` if it did not panic.
    ///
    /// Used by the multibyte regression tests so a panic is reported as a
    /// failed assertion on the surrounding test rather than aborting the
    /// whole test binary.
    fn catch_unwind<F, R>(f: F) -> bool
    where
        F: FnOnce() -> R + std::panic::UnwindSafe,
    {
        std::panic::catch_unwind(f).is_ok()
    }
}
