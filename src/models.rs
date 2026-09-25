use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/// The task status enum, matching the C++ `enum class Status`.
///
/// Serialization uses lowercase snake-case strings (`pending`, `in_progress`,
/// `done`) via explicit `#[serde(rename_all)]` — not serde's default
/// (`InProgress`) — so that `status.yaml` round-trips exactly as the C++
/// binary writes it.
///
/// The [`Display`] impl and [`status_to_string`] emit the *bare* string
/// (e.g. `"pending"`), matching the C++ `statusToString()`.  The bracketed
/// terminal rendering (`[pending]`) is produced by a separate helper that
/// lives in `commands/mod.rs` (the C++ `statusColor()`), not here.
///
/// Deserialization mirrors the C++ `stringToStatus()`: any unrecognised
/// string silently becomes [`Status::Pending`], so hand-edited `status.yaml`
/// files stay readable (reader parity with `storage.cpp:118`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    #[default]
    Pending,
    InProgress,
    Done,
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(string_to_status(&s))
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(status_to_string(*self))
    }
}

impl FromStr for Status {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(string_to_status(s))
    }
}

/// C++ `statusToString(Status s)` — returns the bare YAML/status string.
pub(crate) fn status_to_string(s: Status) -> &'static str {
    match s {
        Status::Pending => "pending",
        Status::InProgress => "in_progress",
        Status::Done => "done",
    }
}

/// C++ `stringToStatus(const std::string& s)` — unknown/empty → `Pending`.
///
/// # Lossy on rewrite (C++ parity)
///
/// The mapping is deliberately forgiving: any unrecognised value (a typo, a
/// status from a newer version, a non-string YAML scalar coerced by the
/// deserializer) reads back as [`Status::Pending`] rather than failing the
/// whole file — matching C++ `storage.cpp:118`, where a hand-edited
/// `status.yaml` must stay readable.
///
/// The cost is that the leniency is **not** reversible. The unknown text only
/// ever lives in the file; the in-memory task is already `Pending`, so the
/// next `write_status_file` persists `status: pending` and the original value
/// is gone. Reading a project with a typo and then writing it back silently
/// normalises that field. Kept as C++ parity rather than rejected, because
/// erroring instead would break every project that has a stale status value.
pub(crate) fn string_to_status(s: &str) -> Status {
    match s {
        "in_progress" => Status::InProgress,
        "done" => Status::Done,
        _ => Status::Pending,
    }
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

/// A single task record as it appears inside `status.yaml`.
///
/// # The `id` field
///
/// `Task.id` is deliberately kept as a struct field with `#[serde(skip)]`: it
/// is never serialized, and is instead re-threaded into it from the
/// `status.yaml` map key by [`storage::read_status_file`] (T004).  This keeps
/// every command seeing the same `Task` shape as the C++ struct, even though
/// the YAML node itself knows nothing about it.  A bare [`Task`] that was not
/// deserialized through [`storage::read_status_file`] will therefore have
/// `id == ""`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct Task {
    /// The task id, equal to the map key in [`StatusFile::tasks`]. Never
    /// serialized; re-threaded from the key by `storage::read_status_file`.
    #[serde(skip)]
    pub(crate) id: String,
    /// The task name. Defaults to `""` when missing (C++: `t["name"] ? ... : ""`).
    #[serde(default)]
    pub(crate) name: String,
    /// The task status. Defaults to [`Status::Pending`].
    #[serde(default)]
    pub(crate) status: Status,
    /// Dependency task ids. Absent, `~`/`null`, `[]`, and flow-style `[T001]`
    /// all normalise to an empty vec; a non-empty vec serializes as a sequence
    /// and an empty vec serializes as `depends: ~` (matching the C++ emitter).
    #[serde(
        default,
        deserialize_with = "de_depends",
        serialize_with = "se_depends"
    )]
    pub(crate) depends: Vec<String>,
    /// Phase number. Defaults to `0`.
    #[serde(default)]
    pub(crate) phase: i32,
    /// Whether the task is on the critical path. Defaults to `false`.
    #[serde(default)]
    pub(crate) critical: bool,
}

fn de_depends<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let opt: Option<Vec<String>> = Option::deserialize(deserializer).map_err(D::Error::custom)?;
    Ok(opt.unwrap_or_default())
}

fn se_depends<S>(depends: &[String], serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if depends.is_empty() {
        // `serde_saphyr::NullableTilde(None)` emits YAML `~` (matching the
        // C++ emitter for an empty `depends` list).
        serde_saphyr::NullableTilde(None::<Vec<String>>).serialize(serializer)
    } else {
        depends.serialize(serializer)
    }
}

// ---------------------------------------------------------------------------
// ProjectConfig
// ---------------------------------------------------------------------------

/// Project configuration, mirroring C++ `ProjectConfig`.
///
/// `phases` is a `BTreeMap` rather than a `HashMap` so that `taskpad status`
/// and `taskpad summary` emit phases in ascending numeric order, matching
/// C++ `std::map` (spec §3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProjectConfig {
    /// Phase number -> phase name. Omitted entirely when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) phases: BTreeMap<i32, String>,
    /// Critical-path task ids. Omitted entirely when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) critical_path: Vec<String>,
}

// ---------------------------------------------------------------------------
// StatusFile
// ---------------------------------------------------------------------------

/// The whole of `status.yaml`, mirrored as a single Rust struct.
///
/// `#[serde(flatten)]` is used on `config` so that `phases` and `critical_path`
/// serialize into the top-level mapping (preserving the C++ `sf.config.phases`
/// access pattern) while keeping the key order `tasks → phases → critical_path`
/// that the writer must match.  It also keeps [`ProjectConfig`] referenced here,
/// preventing a `dead_code` warning under `-D warnings`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StatusFile {
    /// Tasks keyed by id. An absent, `~`/`null` or `{}` value all deserialize
    /// to an empty map; an empty map serializes back as `tasks: ~` (see
    /// [`se_tasks`]).
    #[serde(default, deserialize_with = "de_tasks", serialize_with = "se_tasks")]
    pub(crate) tasks: BTreeMap<String, Task>,
    /// Project config, flattened into the top-level mapping on both serialize
    /// and deserialize.
    #[serde(flatten)]
    pub(crate) config: ProjectConfig,
}

fn de_tasks<'de, D>(deserializer: D) -> std::result::Result<BTreeMap<String, Task>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let opt: Option<BTreeMap<String, Task>> =
        Option::deserialize(deserializer).map_err(D::Error::custom)?;
    Ok(opt.unwrap_or_default())
}

/// Serialize [`StatusFile::tasks`], emitting `~` — not an empty flow map — for
/// an empty task set.
///
/// C++ `writeStatusFile` (`storage.cpp:154-172`) default-constructs
/// `YAML::Node tasksNode`, which is a **Null** node, and assigns it
/// unconditionally with `root["tasks"] = tasksNode`. `YAML::Dump` renders a
/// Null node as `~`, so a project with no tasks writes exactly `tasks: ~` —
/// which is what `taskpad remove T001` on the last task, and any other
/// empty-project write, leaves on disk. A plain `BTreeMap` would instead
/// serialize to the serde default `tasks: {}`, a byte difference in a file the
/// C++ binary also reads and writes, so this is writer parity (Locked
/// decision §4 / ruling #2) rather than a cosmetic preference.
///
/// Non-empty maps serialize as a normal block mapping, unchanged.
fn se_tasks<S>(
    tasks: &BTreeMap<String, Task>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if tasks.is_empty() {
        // `serde_saphyr::NullableTilde(None)` emits YAML `~`, matching the
        // C++ emitter for a Null `tasks` node.
        serde_saphyr::NullableTilde(None::<&BTreeMap<String, Task>>).serialize(serializer)
    } else {
        tasks.serialize(serializer)
    }
}

// ---------------------------------------------------------------------------
// TaskpadError / Result
// ---------------------------------------------------------------------------

/// The crate's error type.
///
/// Currently a single [`Message`] escape hatch (matching the C++ `Result<T>`
/// `value + error-string` wrapper), with room for structured variants to be
/// added opportunistically as the port progresses.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TaskpadError {
    #[error("{0}")]
    Message(String),
}

/// The crate's fallible result alias.
pub(crate) type Result<T> = std::result::Result<T, TaskpadError>;

impl TaskpadError {
    /// Build the invalid-task-ID error, matching the C++ `commands.cpp`
    /// literal byte-for-byte.
    pub(crate) fn invalid_id() -> Self {
        TaskpadError::Message("Invalid task ID format. Expected TXXX (see Task ID Format)".into())
    }

    /// Build the "task not found" error for `id`, matching the C++ literal.
    pub(crate) fn task_not_found(id: &str) -> Self {
        TaskpadError::Message(format!("Task {id} not found"))
    }

    /// Build the missing-`status.yaml` error, matching the C++ literal.
    pub(crate) fn no_status_yaml() -> Self {
        TaskpadError::Message(
            "No status.yaml found. Run 'taskpad import' or 'taskpad new' first".into(),
        )
    }

    /// Build the write-failure error for `path`, matching the C++
    /// `storage.cpp` literal (`Cannot write to <file path>. Check permissions`).
    pub(crate) fn cannot_write(path: &str) -> Self {
        TaskpadError::Message(format!("Cannot write to {path}. Check permissions"))
    }

    /// Build the read-failure error for `path`, carrying the OS reason
    /// verbatim, e.g. `Cannot read specs/tasks/status.yaml: Permission denied
    /// (os error 13)`.
    ///
    /// The `Cannot read …` shape has no C++ counterpart: the C++ readers map
    /// every read failure onto their "not found" message. This is a
    /// deliberate, documented divergence (T020) for the same reason the
    /// reader above is strict: a file that exists but cannot be read is not
    /// a missing file, and reporting it as one lets `taskpad new` start from
    /// an empty project and overwrite a `status.yaml` it merely failed to
    /// read. Only [`std::io::ErrorKind::NotFound`] keeps the original
    /// message; every other kind lands here.
    pub(crate) fn cannot_read(path: &str, err: &std::io::Error) -> Self {
        TaskpadError::Message(format!("Cannot read {path}: {err}"))
    }

    /// Build the missing-task-file error for `path`, matching the C++ literal.
    pub(crate) fn task_file_not_found(path: &str) -> Self {
        TaskpadError::Message(format!("Task file {path} not found"))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_to_string_round_trip() {
        assert_eq!(status_to_string(Status::Pending), "pending");
        assert_eq!(status_to_string(Status::InProgress), "in_progress");
        assert_eq!(status_to_string(Status::Done), "done");
        assert_eq!(format!("{}", Status::Pending), "pending");
        assert_eq!(format!("{}", Status::InProgress), "in_progress");
        assert_eq!(format!("{}", Status::Done), "done");
    }

    #[test]
    fn string_to_status_round_trip() {
        assert_eq!(string_to_status("pending"), Status::Pending);
        assert_eq!(string_to_status("in_progress"), Status::InProgress);
        assert_eq!(string_to_status("done"), Status::Done);
        assert_eq!(string_to_status("unknown"), Status::Pending);
        assert_eq!(string_to_status(""), Status::Pending);
        assert_eq!("pending".parse::<Status>().unwrap(), Status::Pending);
        assert_eq!("in_progress".parse::<Status>().unwrap(), Status::InProgress);
        assert_eq!("done".parse::<Status>().unwrap(), Status::Done);
    }

    #[test]
    fn task_default_values() {
        let t = Task::default();
        assert!(t.id.is_empty());
        assert!(t.name.is_empty());
        assert_eq!(t.status, Status::Pending);
        assert!(t.depends.is_empty());
        assert_eq!(t.phase, 0);
        assert!(!t.critical);
    }

    #[test]
    fn task_with_values() {
        let t = Task {
            id: "T001".to_string(),
            name: "Test Task".to_string(),
            status: Status::InProgress,
            depends: vec!["T002".to_string()],
            phase: 2,
            critical: true,
        };
        assert_eq!(t.id, "T001");
        assert_eq!(t.name, "Test Task");
        assert_eq!(t.status, Status::InProgress);
        assert_eq!(t.depends, vec!["T002"]);
        assert_eq!(t.phase, 2);
        assert!(t.critical);
    }

    #[test]
    fn project_config_round_trip() {
        let mut config = ProjectConfig::default();
        config.phases.insert(0, "Scaffolding".to_string());
        config.critical_path = vec!["T001".to_string()];
        let yaml = serde_saphyr::to_string(&config).unwrap();
        let back: ProjectConfig = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(config, back);
    }

    #[test]
    fn project_config_omits_empty() {
        let config = ProjectConfig::default();
        let yaml = serde_saphyr::to_string(&config).unwrap();
        assert!(!yaml.contains("phases"));
        assert!(!yaml.contains("critical_path"));
    }

    #[test]
    fn status_file_round_trip_restores_ids() {
        let mut sf = StatusFile::default();
        let t1 = Task {
            id: "T001".to_string(),
            name: "Project Setup".to_string(),
            status: Status::Pending,
            depends: vec![],
            phase: 0,
            critical: true,
        };
        let t2 = Task {
            id: "T002".to_string(),
            name: "Asset Acquisition".to_string(),
            status: Status::Pending,
            depends: vec!["T001".to_string()],
            phase: 0,
            critical: false,
        };
        sf.tasks.insert("T001".to_string(), t1);
        sf.tasks.insert("T002".to_string(), t2);
        sf.config.phases.insert(0, "Scaffolding".to_string());
        sf.config.critical_path = vec!["T001".to_string()];

        let yaml = serde_saphyr::to_string(&sf).unwrap();
        let mut back: StatusFile = serde_saphyr::from_str(&yaml).unwrap();

        for (id, task) in back.tasks.iter_mut() {
            task.id = id.clone();
        }

        assert_eq!(back.tasks.len(), 2);
        assert_eq!(back.tasks["T001"].id, "T001");
        assert_eq!(back.tasks["T001"].name, "Project Setup");
        assert_eq!(back.tasks["T001"].status, Status::Pending);
        assert!(back.tasks["T001"].critical);
        assert_eq!(back.tasks["T002"].id, "T002");
        assert_eq!(back.tasks["T002"].name, "Asset Acquisition");
        assert_eq!(back.tasks["T002"].depends, vec!["T001"]);
        assert_eq!(back.config.phases.get(&0), Some(&"Scaffolding".to_string()));
        assert_eq!(back.config.critical_path, vec!["T001"]);
    }

    #[test]
    fn bare_task_round_trip_drops_id() {
        let t = Task {
            name: "Standalone".to_string(),
            status: Status::Done,
            phase: 1,
            critical: false,
            ..Default::default()
        };
        let yaml = serde_saphyr::to_string(&t).unwrap();
        let back: Task = serde_saphyr::from_str(&yaml).unwrap();
        assert!(back.id.is_empty(), "bare Task deserializes with id == \"\"");
        assert_eq!(back.name, "Standalone");
        assert_eq!(back.status, Status::Done);
        assert_eq!(back.phase, 1);
        assert!(!back.critical);
    }

    #[test]
    fn empty_depends_serializes_as_tilde() {
        let t = Task {
            name: "NoDeps".to_string(),
            ..Default::default()
        };
        let yaml = serde_saphyr::to_string(&t).unwrap();
        assert!(
            yaml.contains("depends: ~"),
            "expected `depends: ~`, got:\n{yaml}"
        );
    }

    #[test]
    fn null_and_empty_depends_deserialize_to_empty() {
        for yaml in ["depends: ~", "depends: null", "depends: []"] {
            let input = format!("name: T\n{yaml}\nstatus: pending\nphase: 0\ncritical: false\n");
            let t: Task = serde_saphyr::from_str(&input).unwrap();
            assert!(
                t.depends.is_empty(),
                "yaml {yaml:?} did not produce empty depends"
            );
        }
        let input = "name: T\ndepends: [T001, T002]\nstatus: pending\nphase: 0\ncritical: false\n";
        let t: Task = serde_saphyr::from_str(input).unwrap();
        assert_eq!(t.depends, ["T001", "T002"]);
    }

    // ---- status.yaml writer byte parity (Locked decision §4) ----
    //
    // Every expectation below is a literal transcription of what the C++
    // binary's `YAML::Dump` emitter puts on disk, captured from
    // `../taskpad/taskpad` and re-checked byte-for-byte with `xxd`. These
    // assert the *full* document, not a substring, so a stray key, a reordered
    // key, a different empty-collection spelling, or a trailing newline all
    // fail. `write_status_file` (storage.rs) strips the one trailing `\n`
    // serde-saphyr appends; these model-level expectations keep it and are
    // compared modulo that single newline, with the byte-parity assertions in
    // `storage.rs::tests` covering the on-disk form directly.

    /// Serialize `sf` the way `write_status_file` does — same options, same
    /// trailing-newline strip — so a model-level assertion is the exact file
    /// content.
    fn dump_status_file(sf: &StatusFile) -> String {
        let opts = serde_saphyr::ser_options! {
            compact_list_indent: false,
        };
        serde_saphyr::to_string_with_options(sf, opts)
            .unwrap()
            .trim_end_matches('\n')
            .to_string()
    }

    fn task(name: &str, status: Status, depends: &[&str]) -> Task {
        Task {
            name: name.to_string(),
            status,
            depends: depends.iter().map(|d| d.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_project_serializes_tasks_as_null_not_empty_map() {
        // C++: `YAML::Node tasksNode;` stays Null when no task is assigned, and
        // `root["tasks"] = tasksNode` writes it unconditionally — `YAML::Dump`
        // renders Null as `~`. Reproduced with the C++ binary by removing the
        // only task in a one-task project.
        let sf = StatusFile::default();
        assert_eq!(dump_status_file(&sf), "tasks: ~");
    }

    #[test]
    fn empty_project_with_phases_still_writes_tasks_as_null() {
        // An empty `tasks` map and a non-empty `phases` map coexist: `phases`
        // is still omitted when empty, but never turns a Null `tasks` into a
        // flow map.
        let mut sf = StatusFile::default();
        sf.config.phases.insert(0, "Scaffolding".to_string());
        assert_eq!(dump_status_file(&sf), "tasks: ~\nphases:\n  0: Scaffolding");
    }

    #[test]
    fn single_task_serializes_exact_bytes() {
        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), task("Only Task", Status::Pending, &[]));
        // Captured from `../taskpad/taskpad new "Only Task"`.
        assert_eq!(
            dump_status_file(&sf),
            concat!(
                "tasks:\n",
                "  T001:\n",
                "    name: Only Task\n",
                "    status: pending\n",
                "    depends: ~\n",
                "    phase: 0\n",
                "    critical: false",
            )
        );
    }

    #[test]
    fn multi_task_serializes_exact_bytes_in_id_order() {
        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), task("Task One", Status::Done, &[]));
        sf.tasks.insert(
            "T002".to_string(),
            task("Task Two", Status::InProgress, &["T001"]),
        );
        sf.tasks.insert(
            "T010".to_string(),
            task("Task Ten", Status::Pending, &["T001", "T002"]),
        );
        // Captured from `../taskpad/taskpad new "Task One" && … "Task Two"`,
        // with statuses/depends set via `done`/`do`. Keys ascend as strings
        // (`T001` < `T002` < `T010`), matching the C++ `std::map<std::string>`.
        assert_eq!(
            dump_status_file(&sf),
            concat!(
                "tasks:\n",
                "  T001:\n",
                "    name: Task One\n",
                "    status: done\n",
                "    depends: ~\n",
                "    phase: 0\n",
                "    critical: false\n",
                "  T002:\n",
                "    name: Task Two\n",
                "    status: in_progress\n",
                "    depends:\n",
                "      - T001\n",
                "    phase: 0\n",
                "    critical: false\n",
                "  T010:\n",
                "    name: Task Ten\n",
                "    status: pending\n",
                "    depends:\n",
                "      - T001\n",
                "      - T002\n",
                "    phase: 0\n",
                "    critical: false",
            )
        );
    }

    #[test]
    fn multi_task_with_phases_and_critical_path_serializes_exact_bytes() {
        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), task("Scaffold", Status::Done, &[]));
        let mut t2 = task("Build", Status::Pending, &["T001"]);
        t2.phase = 1;
        t2.critical = true;
        sf.tasks.insert("T002".to_string(), t2);
        sf.config.phases.insert(0, "Scaffolding".to_string());
        sf.config.phases.insert(1, "Building".to_string());
        sf.config.critical_path = vec!["T001".to_string(), "T002".to_string()];
        // `phases` keys are emitted in ascending numeric order and
        // `critical_path` items are indented one step deeper than their key,
        // per yaml-cpp's block-sequence style.
        assert_eq!(
            dump_status_file(&sf),
            concat!(
                "tasks:\n",
                "  T001:\n",
                "    name: Scaffold\n",
                "    status: done\n",
                "    depends: ~\n",
                "    phase: 0\n",
                "    critical: false\n",
                "  T002:\n",
                "    name: Build\n",
                "    status: pending\n",
                "    depends:\n",
                "      - T001\n",
                "    phase: 1\n",
                "    critical: true\n",
                "phases:\n",
                "  0: Scaffolding\n",
                "  1: Building\n",
                "critical_path:\n",
                "  - T001\n",
                "  - T002",
            )
        );
    }

    #[test]
    fn task_with_empty_depends_serializes_depends_as_null() {
        let mut sf = StatusFile::default();
        sf.tasks
            .insert("T001".to_string(), task("No Deps", Status::Pending, &[]));
        // The empty-`depends` null is independent of the empty-`tasks` one: a
        // populated project still spells an empty dependency list `~`, never
        // `[]`.
        assert_eq!(
            dump_status_file(&sf),
            concat!(
                "tasks:\n",
                "  T001:\n",
                "    name: No Deps\n",
                "    status: pending\n",
                "    depends: ~\n",
                "    phase: 0\n",
                "    critical: false",
            )
        );
        // Guard against a `[]` regression sneaking back in.
        assert!(!dump_status_file(&sf).contains("depends: []"));
    }

    #[test]
    fn null_and_empty_map_tasks_deserialize_to_empty() {
        // `tasks: ~` is what the C++ writer emits for an empty project, so the
        // reader must accept it (round-trip) as well as the `tasks: {}` a
        // hand-edit or another tool may produce.
        for yaml in ["tasks: ~\n", "tasks: null\n", "tasks: {}\n"] {
            let sf: StatusFile = serde_saphyr::from_str(yaml).unwrap();
            assert!(
                sf.tasks.is_empty(),
                "yaml {yaml:?} did not produce an empty task map"
            );
        }
        // …and an absent `tasks` key is still an empty project, not an error.
        let sf: StatusFile = serde_saphyr::from_str("phases:\n  0: Setup\n").unwrap();
        assert!(sf.tasks.is_empty());
        assert_eq!(sf.config.phases.get(&0), Some(&"Setup".to_string()));
    }

    #[test]
    fn empty_project_round_trips_through_the_null_spelling() {
        // A project written with zero tasks must read back as the same
        // zero-task project, and re-writing it must not drift to `{}`.
        let written = dump_status_file(&StatusFile::default());
        let back: StatusFile = serde_saphyr::from_str(&written).unwrap();
        assert!(back.tasks.is_empty());
        assert_eq!(dump_status_file(&back), written);
    }
}
