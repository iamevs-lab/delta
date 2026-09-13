use crate::Store;
use delta_core::NodeId;
use std::any::Any;
use std::collections::HashMap;

#[derive(Default)]
pub struct MemoryStore {
    values: HashMap<NodeId, Box<dyn Any + Send + Sync>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_as<T: 'static>(&self, id: NodeId) -> Option<&T> {
        self.values.get(&id).and_then(|v| v.downcast_ref::<T>())
    }
}

impl Store for MemoryStore {
    fn insert(&mut self, id: NodeId, value: Box<dyn Any + Send + Sync>) {
        self.values.insert(id, value);
    }

    fn get(&self, id: NodeId) -> Option<&(dyn Any + Send + Sync)> {
        self.values.get(&id).map(|b| &**b)
    }

    fn remove(&mut self, id: NodeId) {
        self.values.remove(&id);
    }

    fn contains(&self, id: NodeId) -> bool {
        self.values.contains_key(&id)
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn clear(&mut self) {
        self.values.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use delta_core::id::NodeIdAllocator;

    #[test]
    fn roundtrip() {
        let mut s = MemoryStore::new();
        let mut a = NodeIdAllocator::default();
        let id = a.alloc();
        s.insert(id, Box::new(String::from("hi")));
        assert_eq!(s.get_as::<String>(id).map(String::as_str), Some("hi"));
    }
}
