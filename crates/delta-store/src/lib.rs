//! Storage for node outputs.
//!
//! MVP is in-memory. [`PersistentStore`] is a documented interface, not an
//! implementation. Do not treat process restarts as supported.

mod memory;
mod persist;

pub use memory::MemoryStore;
pub use persist::PersistentStore;

use delta_core::NodeId;
use std::any::Any;

pub trait Store: Send + Sync {
    fn insert(&mut self, id: NodeId, value: Box<dyn Any + Send + Sync>);
    fn get(&self, id: NodeId) -> Option<&(dyn Any + Send + Sync)>;
    fn remove(&mut self, id: NodeId);
    fn contains(&self, id: NodeId) -> bool;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn clear(&mut self);
}

pub trait StoreView {
    fn get(&self, id: NodeId) -> Option<&(dyn Any + Send + Sync)>;
}

impl<T: Store> StoreView for T {
    fn get(&self, id: NodeId) -> Option<&(dyn Any + Send + Sync)> {
        Store::get(self, id)
    }
}
