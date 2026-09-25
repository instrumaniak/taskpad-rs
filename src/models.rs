#![allow(dead_code)]

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
pub enum Status {
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
pub fn status_to_string(s: Status) -> &'static str {
    match s {
        Status::Pending => "pending",
        Status::InProgress => "in_progress",
        Status::Done => "done",
    }
}

/// C++ `stringToStatus(const std::string& s)` — unknown/empty → `Pending`.
pub fn string_to_status(s: &str) -> Status {
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
pub struct Task {
    /// The task id, equal to the map key in [`StatusFile::tasks`]. Never
    /// serialized; re-threaded from the key by `storage::read_status_file`.
    #[serde(skip)]
    pub id: String,
    /// The task name. Defaults to `""` when missing (C++: `t["name"] ? ... : ""`).
    #[serde(default)]
    pub name: String,
    /// The task status. Defaults to [`Status::Pending`].
    #[serde(default)]
    pub status: Status,
    /// Dependency task ids. Absent, `~`/`null`, `[]`, and flow-style `[T001]`
    /// all normalise to an empty vec; a non-empty vec serializes as a sequence
    /// and an empty vec serializes as `depends: ~` (matching the C++ emitter).
    #[serde(
        default,
        deserialize_with = "de_depends",
        serialize_with = "se_depends"
    )]
    pub depends: Vec<String>,
    /// Phase number. Defaults to `0`.
    #[serde(default)]
    pub phase: i32,
    /// Whether the task is on the critical path. Defaults to `false`.
    #[serde(default)]
    pub critical: bool,
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
pub struct ProjectConfig {
    /// Phase number -> phase name. Omitted entirely when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub phases: BTreeMap<i32, String>,
    /// Critical-path task ids. Omitted entirely when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub critical_path: Vec<String>,
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
pub struct StatusFile {
    /// Tasks keyed by id. Absent becomes an empty map.
    #[serde(default)]
    pub tasks: BTreeMap<String, Task>,
    /// Project config, flattened into the top-level mapping on both serialize
    /// and deserialize.
    #[serde(flatten)]
    pub config: ProjectConfig,
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
pub enum TaskpadError {
    #[error("{0}")]
    Message(String),
}

/// The crate's fallible result alias.
pub type Result<T> = std::result::Result<T, TaskpadError>;

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
}
