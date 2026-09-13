use crate::workload::Workload;
use delta_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayFile {
    pub version: u32,
    pub workload: Workload,
}

pub fn save_replay(path: &Path, workload: &Workload) -> Result<()> {
    let file = ReplayFile {
        version: 1,
        workload: workload.clone(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| Error::Serialize(e.to_string()))?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_replay(path: &Path) -> Result<ReplayFile> {
    let data = std::fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(|e| Error::Serialize(e.to_string()))
}
