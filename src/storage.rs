#![allow(dead_code)]

use crate::models::Result;
use crate::models::TaskpadError;

pub fn read_task_dir(_project_root: &str) -> Result<String> {
    Err(TaskpadError::Message("not yet implemented".into()))
}
