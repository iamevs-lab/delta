use serde::{Deserialize, Serialize};
use std::fmt;

/// Generational node identity.
///
/// Stale IDs (wrong generation) are errors, not silent reuse.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl NodeId {
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "n{}.{}", self.index, self.generation)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({}.{})", self.index, self.generation)
    }
}

/// Allocates unique [`NodeId`]s. Not itself a graph.
#[derive(Debug, Default)]
pub struct NodeIdAllocator {
    next_index: u32,
}

impl NodeIdAllocator {
    pub fn alloc(&mut self) -> NodeId {
        let index = self.next_index;
        self.next_index = self
            .next_index
            .checked_add(1)
            .expect("node id space exhausted");
        NodeId::new(index, 1)
    }

    pub fn next_generation(id: NodeId) -> NodeId {
        NodeId::new(
            id.index,
            id.generation
                .checked_add(1)
                .expect("node generation exhausted"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_ordered() {
        let mut a = NodeIdAllocator::default();
        let x = a.alloc();
        let y = a.alloc();
        assert_ne!(x, y);
        assert!(x < y);
    }
}
