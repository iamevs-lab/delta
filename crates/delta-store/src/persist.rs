use crate::Store;
use delta_core::{Error, NodeId, Result};
use std::any::Any;
use std::path::{Path, PathBuf};

/// Persistence is **not** part of the MVP.
///
/// The type exists so adapters can depend on a stable name later.
/// Every method currently returns [`Error::Unsupported`].
pub struct PersistentStore {
    #[allow(dead_code)]
    path: PathBuf,
}

impl PersistentStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let _ = path;
        Err(Error::Unsupported(
            "persistence is deferred (phase 14); use MemoryStore".into(),
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Store for PersistentStore {
    fn insert(&mut self, _id: NodeId, _value: Box<dyn Any + Send + Sync>) {}
    fn get(&self, _id: NodeId) -> Option<&(dyn Any + Send + Sync)> {
        None
    }
    fn remove(&mut self, _id: NodeId) {}
    fn contains(&self, _id: NodeId) -> bool {
        false
    }
    fn len(&self) -> usize {
        0
    }
    fn clear(&mut self) {}
}
