//! Shared harness for the taskpad E2E tier.
//!
//! This is a transcription of the Node.js `tests/helpers.mjs` this tier replaces.
//! The `.mjs` suite was copied verbatim from the C++ repo, so every literal it
//! asserted is a C++ literal — that is why the assertions here are transcribed
//! unchanged rather than "improved". If a ported assertion needs weakening to
//! pass, that is a bug in the Rust port (see `spec.testing.md` §1).
//!
//! Layout follows fd's `tests/testenv/mod.rs`: an environment object owns a
//! `TempDir` plus the fixture files, and assertion methods drive the binary
//! against it. `TempDir`'s `Drop` replaces the old harness's explicit
//! `destroy()`.

#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output as RawOutput, Stdio};
use std::sync::OnceLock;

use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Binary discovery
// ---------------------------------------------------------------------------

/// Path to the `taskpad` binary under test.
///
/// Resolution order: an explicit `TASKPAD_BIN` override, then the runtime
/// `CARGO_BIN_EXE_taskpad`, then the compile-time one. The override exists so
/// the C++ parity cross-check stays reproducible:
///
/// ```text
/// TASKPAD_BIN=../taskpad/taskpad cargo test --test e2e
/// ```
///
/// A *relative* override is canonicalized here, on first use, while the test
/// process is still in the package root (Cargo sets it there) — `run` only ever
/// changes the *child's* working directory, so the test process itself never
/// moves. Without this, the relative path would be resolved from the temp
/// project and every test would fail to spawn the binary.
pub fn binary() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        if let Some(raw) = std::env::var_os("TASKPAD_BIN") {
            let path = PathBuf::from(raw);
            return fs::canonicalize(&path).unwrap_or(path);
        }
        if let Some(raw) = std::env::var_os("CARGO_BIN_EXE_taskpad") {
            return PathBuf::from(raw);
        }
        PathBuf::from(env!("CARGO_BIN_EXE_taskpad"))
    })
}

// ---------------------------------------------------------------------------
// Captured output
// ---------------------------------------------------------------------------

/// The result of one `taskpad` invocation.
///
/// Both streams are `trim()`ed, mirroring `helpers.mjs` — that trim is why the
/// 118 `assert.equal` calls transcribe one-for-one.
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    /// Exit code. A `None` means the process was killed by a signal, which no
    /// assertion in this suite expects — so `.mjs`'s `assert.equal(r.status, 0)`
    /// becomes `assert_eq!(r.code, Some(0))`.
    pub code: Option<i32>,
}

impl Output {
    fn capture(raw: RawOutput) -> Output {
        Output {
            stdout: String::from_utf8_lossy(&raw.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&raw.stderr).trim().to_string(),
            code: raw.status.code(),
        }
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// What a project should be seeded with. The four variants are the four modes
/// `createProject({ … })` supported.
pub enum Fixture {
    /// Just `tasks/` and `.taskpad` — no `status.yaml`, no `T*.md` files.
    /// Covers both `createProject({})` and `createProject({ empty: true })`,
    /// which the `.mjs` helper treated identically.
    Empty,
    /// A literal `status.yaml`, written byte-for-byte. No `T*.md` files.
    StatusYaml(&'static str),
    /// `tasks:` mode — a `status.yaml` is generated *and* a `T*.md` per task.
    Tasks(Vec<TaskSpec>),
    /// `importTasks:` mode — only the `T*.md` files exist; the test drives
    /// `taskpad import` to build `status.yaml` from them.
    Import(Vec<ImportSpec>),
}

/// A task for [`Fixture::Tasks`].
///
/// `name` is optional because `helpers.mjs` derived it from the id (`T002` →
/// `Task 2`) unless the caller overrode it.
#[derive(Clone, Debug)]
pub struct TaskSpec {
    pub id: String,
    pub name: Option<String>,
    pub status: String,
    pub depends: Vec<String>,
    pub phase: u32,
    pub critical: bool,
}

impl Default for TaskSpec {
    fn default() -> TaskSpec {
        TaskSpec {
            id: String::new(),
            name: None,
            status: "pending".to_string(),
            depends: Vec::new(),
            phase: 0,
            critical: false,
        }
    }
}

impl TaskSpec {
    /// A task with every field defaulted but its id — the `.mjs` `'T001'`
    /// string shorthand.
    pub fn new(id: &str) -> TaskSpec {
        TaskSpec {
            id: id.to_string(),
            ..TaskSpec::default()
        }
    }

    pub fn name(mut self, name: &str) -> TaskSpec {
        self.name = Some(name.to_string());
        self
    }

    pub fn status(mut self, status: &str) -> TaskSpec {
        self.status = status.to_string();
        self
    }

    pub fn depends(mut self, depends: &[&str]) -> TaskSpec {
        self.depends = depends.iter().map(|d| d.to_string()).collect();
        self
    }

    pub fn phase(mut self, phase: u32) -> TaskSpec {
        self.phase = phase;
        self
    }

    pub fn critical(mut self, critical: bool) -> TaskSpec {
        self.critical = critical;
        self
    }

    /// The name as written to disk, falling back to `helpers.mjs`'s
    /// `Task <n>` derivation from the numeric part of the id.
    pub fn display_name(&self) -> String {
        match &self.name {
            Some(name) => name.clone(),
            None => {
                let n: u32 = self
                    .id
                    .trim_start_matches('T')
                    .parse()
                    .expect("task ids in fixtures are always T<digits>");
                format!("Task {n}")
            }
        }
    }
}

/// A task file for [`Fixture::Import`].
///
/// `phase` / `critical` / `status` are `Option` because `helpers.mjs` emitted
/// the `## Phase:` / `## Critical:` / `## Status:` sections *only* when the
/// caller set them — and the suite asserts on their absence.
#[derive(Clone, Debug, Default)]
pub struct ImportSpec {
    pub id: String,
    pub name: Option<String>,
    pub phase: Option<u32>,
    pub critical: Option<bool>,
    pub status: Option<String>,
    pub depends: Vec<String>,
}

impl ImportSpec {
    /// A task file with every optional header omitted.
    pub fn new(id: &str) -> ImportSpec {
        ImportSpec {
            id: id.to_string(),
            ..ImportSpec::default()
        }
    }

    pub fn name(mut self, name: &str) -> ImportSpec {
        self.name = Some(name.to_string());
        self
    }

    pub fn depends(mut self, depends: &[&str]) -> ImportSpec {
        self.depends = depends.iter().map(|d| d.to_string()).collect();
        self
    }

    pub fn phase(mut self, phase: u32) -> ImportSpec {
        self.phase = Some(phase);
        self
    }

    pub fn critical(mut self, critical: bool) -> ImportSpec {
        self.critical = Some(critical);
        self
    }

    pub fn status(mut self, status: &str) -> ImportSpec {
        self.status = Some(status.to_string());
        self
    }

    pub fn display_name(&self) -> String {
        match &self.name {
            Some(name) => name.clone(),
            None => {
                let n: u32 = self
                    .id
                    .trim_start_matches('T')
                    .parse()
                    .expect("task ids in fixtures are always T<digits>");
                format!("Task {n}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture builders (transcribed from helpers.mjs)
// ---------------------------------------------------------------------------

/// `helpers.mjs` `toKebab`: lowercase, whitespace runs collapsed to `-`.
pub fn to_kebab(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    for ch in s.chars().flat_map(char::to_lowercase) {
        if ch.is_whitespace() {
            in_space = true;
        } else {
            if in_space {
                out.push('-');
                in_space = false;
            }
            out.push(ch);
        }
    }
    if in_space {
        out.push('-');
    }
    out
}

/// `helpers.mjs` `taskFilename`: `T002-second-task.md`.
pub fn task_filename(id: &str, name: &str) -> String {
    format!("{}-{}.md", id, to_kebab(name))
}

fn dep_line(depends: &[String]) -> String {
    if depends.is_empty() {
        "(None)".to_string()
    } else {
        depends.join(", ")
    }
}

/// `helpers.mjs` `mdContent` — the body a `tasks:` fixture gets.
///
/// Note this is the *fixture* style, not what the Rust writer emits: `depends`
/// is written inline as `[]` / `[T001]` here. Several assertions pin exactly
/// that, so it must not be "corrected".
pub fn md_content(id: &str, name: &str, depends: &[String]) -> String {
    [
        format!("# {id}: {name}"),
        String::new(),
        "## Goal".to_string(),
        String::new(),
        "Test task.".to_string(),
        String::new(),
        "## Depends On".to_string(),
        String::new(),
        dep_line(depends),
        String::new(),
        "## Implementation Steps".to_string(),
        String::new(),
        "1. Implement the task".to_string(),
        String::new(),
        "## Acceptance Criteria".to_string(),
        String::new(),
        "- [ ] Task complete".to_string(),
        String::new(),
    ]
    .join("\n")
}

/// `helpers.mjs` `mdImportContent` — the body an `importTasks:` fixture gets.
///
/// The optional headers are spliced in at index 3 one after another, exactly as
/// the `.mjs` did, so the emitted order is `## Status:`, `## Critical:`,
/// `## Phase:`, `## Goal`.
pub fn md_import_content(id: &str, name: &str, spec: &ImportSpec) -> String {
    let mut lines: Vec<String> = vec![
        format!("# {id}: {name}"),
        String::new(),
        "## Goal".to_string(),
        String::new(),
        "Test task.".to_string(),
        String::new(),
        "## Depends On".to_string(),
        String::new(),
        dep_line(&spec.depends),
        String::new(),
        "## Spec References".to_string(),
        String::new(),
        "- `specs/spec1.md`".to_string(),
        String::new(),
        "## Files to Create/Modify".to_string(),
        String::new(),
        format!("- `src/{}.cpp`", to_kebab(name)),
        String::new(),
        "## Implementation Steps".to_string(),
        String::new(),
        "1. Implement the task".to_string(),
        String::new(),
        "## Acceptance Criteria".to_string(),
        String::new(),
        "- [ ] Task complete".to_string(),
        String::new(),
    ];
    if let Some(phase) = spec.phase {
        lines.splice(3..3, [String::new(), format!("## Phase: {phase}")]);
    }
    if let Some(critical) = spec.critical {
        lines.splice(3..3, [String::new(), format!("## Critical: {critical}")]);
    }
    if let Some(status) = &spec.status {
        lines.splice(3..3, [String::new(), format!("## Status: {status}")]);
    }
    lines.join("\n")
}

/// `helpers.mjs` `generateYaml` — the `tasks:`-mode `status.yaml`.
///
/// Deliberately *not* the byte-compatible writer format: the header comment,
/// the quoted `name`, and the inline `depends: [...]` are all fixture style.
pub fn generate_yaml(tasks: &[TaskSpec]) -> String {
    let mut yaml = String::from("# taskpad status file\n\ntasks:\n");
    for t in tasks {
        yaml.push_str(&format!("  {}:\n", t.id));
        yaml.push_str(&format!("    name: \"{}\"\n", t.display_name()));
        yaml.push_str(&format!("    status: {}\n", t.status));
        yaml.push_str(&format!("    depends: [{}]\n", t.depends.join(", ")));
        yaml.push_str(&format!("    phase: {}\n", t.phase));
        yaml.push_str(&format!("    critical: {}\n", t.critical));
    }
    yaml
}

// ---------------------------------------------------------------------------
// The project environment
// ---------------------------------------------------------------------------

/// One isolated project directory plus the `taskpad` binary to drive it.
///
/// Each test owns its own `Project`, so the suite is safe under `cargo test`'s
/// parallel execution.
pub struct Project {
    _tmp: TempDir,
    root: PathBuf,
}

impl Project {
    /// Create a temp project and seed it. The `tasks/` directory and the
    /// `.taskpad` config always exist first — `.taskpad` is *not* created by
    /// `taskpad init` itself, so `init` tests unlink it before running.
    pub fn new(fixture: Fixture) -> Project {
        let tmp = TempDir::new().expect("temp dir");
        let root = tmp.path().to_path_buf();
        let tasks = root.join("tasks");
        fs::create_dir_all(&tasks).expect("create tasks dir");
        fs::write(root.join(".taskpad"), "task-dir: tasks\n").expect("write .taskpad");

        match fixture {
            Fixture::Empty => {}
            Fixture::StatusYaml(yaml) => {
                fs::write(tasks.join("status.yaml"), yaml).expect("write status.yaml");
            }
            Fixture::Tasks(specs) => {
                fs::write(tasks.join("status.yaml"), generate_yaml(&specs))
                    .expect("write status.yaml");
                for t in &specs {
                    let name = t.display_name();
                    fs::write(
                        tasks.join(task_filename(&t.id, &name)),
                        md_content(&t.id, &name, &t.depends),
                    )
                    .expect("write task file");
                }
            }
            Fixture::Import(specs) => {
                for t in &specs {
                    let name = t.display_name();
                    fs::write(
                        tasks.join(task_filename(&t.id, &name)),
                        md_import_content(&t.id, &name, t),
                    )
                    .expect("write task file");
                }
            }
        }

        Project { _tmp: tmp, root }
    }

    /// Run `taskpad` in the project root and capture its output.
    ///
    /// `Command::output()` gives the child a **null stdin**, so a command that
    /// prompts in a non-interactive call gets EOF instead of hanging. The
    /// `.mjs` suite inherited node's stdin here, which was a latent hang.
    pub fn run(&self, args: &[&str]) -> Output {
        let raw = Command::new(binary())
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap_or_else(|e| panic!("failed to spawn {:?} with {args:?}: {e}", binary()));
        Output::capture(raw)
    }

    /// Run `taskpad` with `input` on stdin — for the interactive `[y/N]`
    /// prompt in `remove`.
    ///
    /// Note the argument order: input first, then the argv. The `.mjs` took the
    /// input as the *last* variadic argument, which does not translate.
    pub fn run_interactive(&self, input: &str, args: &[&str]) -> Output {
        let mut child = Command::new(binary())
            .args(args)
            .current_dir(&self.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {:?} with {args:?}: {e}", binary()));
        child
            .stdin
            .as_mut()
            .expect("piped stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
        let raw = child.wait_with_output().expect("wait for taskpad");
        Output::capture(raw)
    }

    /// Absolute path of `rel` inside the project — the `.mjs` `resolve()`.
    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    pub fn read(&self, rel: &str) -> String {
        let path = self.path(rel);
        fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
    }

    pub fn write(&self, rel: &str, contents: &str) {
        let path = self.path(rel);
        fs::write(&path, contents)
            .unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.path(rel).exists()
    }

    /// Entry names in `rel`, sorted. `fs::read_dir` order is arbitrary, so
    /// every listing assertion goes through this.
    pub fn read_dir_sorted(&self, rel: &str) -> Vec<String> {
        let path = self.path(rel);
        let mut names: Vec<String> = fs::read_dir(&path)
            .unwrap_or_else(|e| panic!("failed to read dir {}: {e}", path.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|e| panic!("failed to read entry in {}: {e}", path.display()))
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    pub fn remove(&self, rel: &str) {
        let path = self.path(rel);
        fs::remove_file(&path)
            .unwrap_or_else(|e| panic!("failed to remove {}: {e}", path.display()));
    }

    /// Set a file or directory's mode. Used by the unwritable-directory test.
    #[cfg(unix)]
    pub fn set_mode(&self, rel: &str, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        let path = self.path(rel);
        fs::set_permissions(&path, fs::Permissions::from_mode(mode))
            .unwrap_or_else(|e| panic!("failed to chmod {}: {e}", path.display()));
    }
}

// ---------------------------------------------------------------------------
// Assertions
//
// One helper per *form* the `.mjs` regexes took, so a transcribed assertion
// reads like the original. `usize::MAX` is the "unbounded gap" sentinel.
// ---------------------------------------------------------------------------

/// Core ordered-subsequence search. The gap *between* consecutive fragments is
/// bounded by `max_gap`; the first fragment may sit anywhere.
pub fn ordered_from(hay: &str, frags: &[&str], max_gap: usize) -> bool {
    let mut pos = 0;
    for (i, frag) in frags.iter().enumerate() {
        let Some(at) = hay.get(pos..).and_then(|rest| rest.find(frag)) else {
            return false;
        };
        if i > 0 && at > max_gap {
            return false;
        }
        pos += at + frag.len();
    }
    true
}

fn report(what: &str, hay: &str) -> String {
    format!("expected {what}\n--- actual ---\n{hay}\n--------------")
}

/// `assert.match(hay, /lit/)` — a pattern with no metacharacters.
pub fn contains(hay: &str, needle: &str) {
    assert!(
        hay.contains(needle),
        "{}",
        report(&format!("{needle:?}"), hay)
    );
}

/// `assert.doesNotMatch(hay, /lit/)`.
pub fn not_contains(hay: &str, needle: &str) {
    assert!(
        !hay.contains(needle),
        "{}",
        report(&format!("no {needle:?}"), hay)
    );
}

/// `assert.match(hay, /^…$/m)` — some line is exactly `line`.
pub fn has_line(hay: &str, line: &str) {
    assert!(
        hay.lines().any(|l| l == line),
        "{}",
        report(&format!("a line exactly {line:?}"), hay)
    );
}

/// `assert.doesNotMatch(hay, /^…$/m)`.
pub fn not_has_line(hay: &str, line: &str) {
    assert!(
        !hay.lines().any(|l| l == line),
        "{}",
        report(&format!("no line exactly {line:?}"), hay)
    );
}

/// `assert.match(hay, /^…/m)` — some line starts with `prefix`.
pub fn line_starts_with(hay: &str, prefix: &str) {
    assert!(
        hay.lines().any(|l| l.starts_with(prefix)),
        "{}",
        report(&format!("a line starting {prefix:?}"), hay)
    );
}

/// `assert.doesNotMatch(hay, /^…/m)`.
pub fn not_line_starts_with(hay: &str, prefix: &str) {
    assert!(
        !hay.lines().any(|l| l.starts_with(prefix)),
        "{}",
        report(&format!("no line starting {prefix:?}"), hay)
    );
}

/// `assert.match(hay, /a.*b/)` — the fragments share one line, in order.
pub fn has_line_ordered(hay: &str, frags: &[&str]) {
    assert!(
        hay.lines().any(|l| ordered_from(l, frags, usize::MAX)),
        "{}",
        report(&format!("one line holding {frags:?} in order"), hay)
    );
}

/// `assert.match(hay, /a[\s\S]*b/)` — fragments in order, anywhere in the text.
pub fn has_ordered(hay: &str, frags: &[&str]) {
    assert!(
        ordered_from(hay, frags, usize::MAX),
        "{}",
        report(&format!("{frags:?} in order"), hay)
    );
}

/// `assert.match(hay, /a[\s\S]{0,n}b/)` — like [`has_ordered`], but the gap
/// after the first fragment is capped at `n` characters.
pub fn has_ordered_within(hay: &str, frags: &[&str], max_gap: usize) {
    assert!(
        ordered_from(hay, frags, max_gap),
        "{}",
        report(&format!("{frags:?} in order within {max_gap} chars"), hay)
    );
}

/// `assert.match(hay, /head[\s\S]*mid[\s\S]{0,max_gap}tail/)` — a *mixed* form
/// where only the final gap is bounded.
///
/// [`has_ordered_within`] cannot express this: it caps every consecutive gap,
/// but here the gap between `head` and `mid` is unbounded and only the
/// `mid`→`tail` gap is limited. Searching every `head` and, within it, every
/// `mid` reproduces exactly what the regex engine's backtracking does — neither
/// weaker nor stricter.
pub fn has_ordered_bounded_tail(hay: &str, head: &str, mid: &str, tail: &str, max_gap: usize) {
    let found = hay.match_indices(head).any(|(at, _)| {
        let rest = &hay[at + head.len()..];
        rest.match_indices(mid).any(|(at, _)| {
            let after = &rest[at + mid.len()..];
            after.find(tail).is_some_and(|gap| gap <= max_gap)
        })
    });
    assert!(
        found,
        "{}",
        report(
            &format!("{head:?} then {mid:?} then {tail:?} within {max_gap} chars"),
            hay
        )
    );
}

/// `assert.match(hay, /head\n.*tail/)` — some line holds `head`, and the line
/// immediately after it holds `tail`. Keeps `.*` (no newline) honest.
pub fn has_line_followed_by(hay: &str, head: &str, tail: &str) {
    let lines: Vec<&str> = hay.lines().collect();
    let found = lines.iter().enumerate().any(|(i, line)| {
        line.contains(head) && lines.get(i + 1).is_some_and(|next| next.contains(tail))
    });
    assert!(found, "{}", report(&format!("{head:?} then {tail:?}"), hay));
}

/// `assert.match(hay, /pre\d+post/)`.
pub fn has_digits_between(hay: &str, pre: &str, post: &str) {
    let found = hay.match_indices(pre).any(|(at, _)| {
        let rest = &hay[at + pre.len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && rest[digits..].starts_with(post)
    });
    assert!(
        found,
        "{}",
        report(&format!("{pre:?} digits {post:?}"), hay)
    );
}

/// `assert.match(hay, /pre\S/)` — `pre` followed by a non-whitespace character.
pub fn has_nonspace_after(hay: &str, pre: &str) {
    let found = hay.match_indices(pre).any(|(at, _)| {
        hay[at + pre.len()..]
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace())
    });
    assert!(found, "{}", report(&format!("{pre:?} then non-space"), hay));
}

/// `assert.match(hay, /lit/i)` — ASCII case-insensitive substring.
pub fn contains_ignore_ascii_case(hay: &str, needle: &str) {
    assert!(
        hay.to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase()),
        "{}",
        report(&format!("{needle:?} ignoring ASCII case"), hay)
    );
}

fn digits_at(bytes: &[u8], at: usize, n: usize) -> bool {
    bytes.len() >= at + n && bytes[at..at + n].iter().all(u8::is_ascii_digit)
}

/// `assert.match(hay, /- \[\d{4}-\d{2}-\d{2} \d{2}:\d{2}\] message/)` — the
/// timestamped bullet `taskpad log` appends under `## Notes`.
pub fn has_log_entry(hay: &str, message: &str) {
    let found = hay.lines().any(|line| {
        let Some(rest) = line.strip_prefix("- [") else {
            return false;
        };
        let b = rest.as_bytes();
        // YYYY-MM-DD HH:MM] <message>
        digits_at(b, 0, 4)
            && b.get(4) == Some(&b'-')
            && digits_at(b, 5, 2)
            && b.get(7) == Some(&b'-')
            && digits_at(b, 8, 2)
            && b.get(10) == Some(&b' ')
            && digits_at(b, 11, 2)
            && b.get(13) == Some(&b':')
            && digits_at(b, 14, 2)
            && b.get(16) == Some(&b']')
            && rest.get(18..) == Some(message)
    });
    assert!(
        found,
        "{}",
        report(&format!("a log entry {message:?}"), hay)
    );
}
