use crate::invalidate::{invalidate, InvalidationMode, InvalidationReport};
use crate::limits::GraphLimits;
use crate::stats::GraphStats;
use crate::topo::{self, Schedule};
use delta_core::{Error, Fingerprint, InvalidationReason, NodeId, NodeStatus, Result, Version};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: NodeId,
    pub name: String,
    pub kind: String,
    pub status: NodeStatus,
    pub version: Version,
    pub input_fingerprint: Option<Fingerprint>,
    pub output_fingerprint: Option<Fingerprint>,
    pub dependencies: Vec<NodeId>,
    pub dependents: Vec<NodeId>,
    pub last_invalidation: Option<InvalidationReason>,
    pub last_compute_nanos: u128,
    pub last_recompute_count: u64,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeInspect {
    pub id: NodeId,
    pub name: String,
    pub kind: String,
    pub status: NodeStatus,
    pub version: Version,
    pub input_fingerprint: Option<String>,
    pub output_fingerprint: Option<String>,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub last_invalidation: Option<String>,
    pub last_compute_nanos: u128,
    pub last_recompute_count: u64,
    pub metadata: BTreeMap<String, String>,
}

struct Slot {
    generation: u32,
    live: bool,
    data: Option<NodeRecord>,
}

pub struct Graph {
    slots: Vec<Slot>,
    free: Vec<u32>,
    limits: GraphLimits,
    edge_count: usize,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            limits: GraphLimits::default(),
            edge_count: 0,
        }
    }

    pub fn with_limits(limits: GraphLimits) -> Self {
        Self {
            limits,
            ..Self::new()
        }
    }

    pub fn limits(&self) -> &GraphLimits {
        &self.limits
    }

    pub fn insert(&mut self, name: impl Into<String>, kind: impl Into<String>) -> Result<NodeId> {
        let live = self.live_count();
        if live >= self.limits.max_nodes {
            return Err(Error::Limit(format!(
                "node count {live} exceeds {}",
                self.limits.max_nodes
            )));
        }

        let (index, generation) = if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.generation = slot.generation.saturating_add(1).max(1);
            slot.live = true;
            (index, slot.generation)
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 1,
                live: true,
                data: None,
            });
            (index, 1)
        };

        let id = NodeId::new(index, generation);
        self.slots[index as usize].data = Some(NodeRecord {
            id,
            name: name.into(),
            kind: kind.into(),
            status: NodeStatus::Pending,
            version: Version::NEVER,
            input_fingerprint: None,
            output_fingerprint: None,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            last_invalidation: None,
            last_compute_nanos: 0,
            last_recompute_count: 0,
            metadata: BTreeMap::new(),
        });
        Ok(id)
    }

    pub fn remove(&mut self, id: NodeId) -> Result<NodeRecord> {
        let rec = self.node(id)?.clone();
        for dep in rec.dependencies.clone() {
            self.remove_dependency(id, dep)?;
        }
        for depent in rec.dependents.clone() {
            self.remove_dependency(depent, id)?;
        }
        let slot = self.slot_mut(id)?;
        let taken = slot.data.take().ok_or(Error::UnknownNode(id))?;
        slot.live = false;
        self.free.push(id.index());
        Ok(taken)
    }

    pub fn node(&self, id: NodeId) -> Result<&NodeRecord> {
        let slot = self.slot(id)?;
        slot.data.as_ref().ok_or(Error::UnknownNode(id))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Result<&mut NodeRecord> {
        let slot = self.slot_mut(id)?;
        slot.data.as_mut().ok_or(Error::UnknownNode(id))
    }

    pub fn live_ids(&self) -> Vec<NodeId> {
        self.slots
            .iter()
            .filter(|s| s.live)
            .filter_map(|s| s.data.as_ref().map(|n| n.id))
            .collect()
    }

    pub fn live_count(&self) -> usize {
        self.slots.iter().filter(|s| s.live).count()
    }

    /// `node` depends on `dependency` (data flows dependency → node).
    pub fn add_dependency(&mut self, node: NodeId, dependency: NodeId) -> Result<()> {
        if node == dependency {
            return Err(Error::Cycle(format!("{node} depends on itself")));
        }
        self.node(node)?;
        self.node(dependency)?;
        if self.reaches(node, dependency)? {
            return Err(Error::Cycle(format!(
                "adding {node} → depends on {dependency} would cycle"
            )));
        }
        if self.edge_count >= self.limits.max_edges {
            return Err(Error::Limit(format!(
                "edge count {} exceeds {}",
                self.edge_count, self.limits.max_edges
            )));
        }

        {
            let rec = self.node_mut(node)?;
            if rec.dependencies.contains(&dependency) {
                return Ok(());
            }
            rec.dependencies.push(dependency);
            rec.dependencies.sort();
        }
        {
            let rec = self.node_mut(dependency)?;
            if !rec.dependents.contains(&node) {
                rec.dependents.push(node);
                rec.dependents.sort();
            }
        }
        self.edge_count += 1;
        Ok(())
    }

    pub fn remove_dependency(&mut self, node: NodeId, dependency: NodeId) -> Result<()> {
        let rec = self.node_mut(node)?;
        let before = rec.dependencies.len();
        rec.dependencies.retain(|d| *d != dependency);
        if rec.dependencies.len() != before {
            self.edge_count = self.edge_count.saturating_sub(1);
        }
        let rec = self.node_mut(dependency)?;
        rec.dependents.retain(|d| *d != node);
        Ok(())
    }

    /// True if `from` can reach `to` by following dependents (downstream).
    pub fn reaches(&self, from: NodeId, to: NodeId) -> Result<bool> {
        let mut stack = vec![from];
        let mut seen = std::collections::BTreeSet::new();
        let mut steps = 0usize;
        let limit = self.limits.max_nodes.saturating_mul(2);
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            if id == to && seen.len() > 1 {
                return Ok(true);
            }
            steps += 1;
            if steps > limit {
                return Err(Error::Limit("graph traversal budget exceeded".into()));
            }
            stack.extend(self.node(id)?.dependents.iter().copied());
        }
        Ok(false)
    }

    pub fn invalidate(
        &mut self,
        seed: NodeId,
        include_seed: bool,
        mode: InvalidationMode,
        reason: InvalidationReason,
    ) -> Result<InvalidationReport> {
        invalidate(self, seed, include_seed, mode, reason)
    }

    pub fn schedule_dirty(&self) -> Result<Schedule> {
        topo::topological_dirty(self)
    }

    pub fn max_depth(&self) -> usize {
        let mut best = 0usize;
        for id in self.live_ids() {
            best = best.max(self.depth_of(id));
        }
        best
    }

    fn depth_of(&self, id: NodeId) -> usize {
        let mut best = 0usize;
        let mut stack = vec![(id, 0usize)];
        let mut seen = std::collections::BTreeSet::new();
        while let Some((id, depth)) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            best = best.max(depth);
            if let Ok(n) = self.node(id) {
                for d in &n.dependencies {
                    stack.push((*d, depth + 1));
                }
            }
        }
        best
    }

    pub fn stats(&self) -> GraphStats {
        let mut s = GraphStats {
            total_nodes: self.live_count(),
            dependency_edges: self.edge_count,
            max_dependency_depth: self.max_depth(),
            ..Default::default()
        };
        let mut max_fanout = 0usize;
        for id in self.live_ids() {
            if let Ok(n) = self.node(id) {
                max_fanout = max_fanout.max(n.dependents.len());
                match n.status {
                    NodeStatus::Dirty | NodeStatus::Computing => s.dirty_nodes += 1,
                    NodeStatus::Clean => s.clean_nodes += 1,
                    NodeStatus::Failed => s.failed_nodes += 1,
                    NodeStatus::Pending => s.pending_nodes += 1,
                }
            }
        }
        s.max_fanout = max_fanout;
        s
    }

    pub fn inspect(&self, id: NodeId) -> Result<NodeInspect> {
        let n = self.node(id)?;
        Ok(NodeInspect {
            id: n.id,
            name: n.name.clone(),
            kind: n.kind.clone(),
            status: n.status,
            version: n.version,
            input_fingerprint: n.input_fingerprint.map(|f| f.to_string()),
            output_fingerprint: n.output_fingerprint.map(|f| f.to_string()),
            dependencies: n
                .dependencies
                .iter()
                .filter_map(|d| self.node(*d).ok().map(|x| x.name.clone()))
                .collect(),
            dependents: n
                .dependents
                .iter()
                .filter_map(|d| self.node(*d).ok().map(|x| x.name.clone()))
                .collect(),
            last_invalidation: n.last_invalidation.as_ref().map(|r| r.to_string()),
            last_compute_nanos: n.last_compute_nanos,
            last_recompute_count: n.last_recompute_count,
            metadata: n.metadata.clone(),
        })
    }

    fn slot(&self, id: NodeId) -> Result<&Slot> {
        let slot = self
            .slots
            .get(id.index() as usize)
            .ok_or(Error::UnknownNode(id))?;
        if !slot.live || slot.generation != id.generation() {
            return Err(Error::StaleNode(id));
        }
        Ok(slot)
    }

    fn slot_mut(&mut self, id: NodeId) -> Result<&mut Slot> {
        let gen = id.generation();
        let slot = self
            .slots
            .get_mut(id.index() as usize)
            .ok_or(Error::UnknownNode(id))?;
        if !slot.live || slot.generation != gen {
            return Err(Error::StaleNode(id));
        }
        Ok(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_rejected() {
        let mut g = Graph::new();
        let a = g.insert("a", "t").unwrap();
        let b = g.insert("b", "t").unwrap();
        g.add_dependency(b, a).unwrap();
        let err = g.add_dependency(a, b).unwrap_err();
        assert!(matches!(err, Error::Cycle(_)));
    }

    #[test]
    fn stale_id_after_remove() {
        let mut g = Graph::new();
        let a = g.insert("a", "t").unwrap();
        g.remove(a).unwrap();
        assert!(matches!(g.node(a), Err(Error::StaleNode(_))));
    }

    #[test]
    fn eager_invalidation_fanout() {
        let mut g = Graph::new();
        let src = g.insert("src", "input").unwrap();
        let t = g.insert("tok", "lex").unwrap();
        let p = g.insert("ast", "parse").unwrap();
        g.add_dependency(t, src).unwrap();
        g.add_dependency(p, t).unwrap();
        let report = g
            .invalidate(
                src,
                false,
                InvalidationMode::Eager,
                InvalidationReason::InputChanged,
            )
            .unwrap();
        assert_eq!(report.fanout, 2);
        assert_eq!(g.node(t).unwrap().status, NodeStatus::Dirty);
        assert_eq!(g.node(p).unwrap().status, NodeStatus::Dirty);
        assert_eq!(g.node(src).unwrap().status, NodeStatus::Pending);
    }
}
