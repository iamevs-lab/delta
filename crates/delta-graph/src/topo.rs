use crate::graph::Graph;
use delta_core::{Error, NodeId, Result};

/// Deterministic dirty-set schedule: increasing node index among ready nodes.
#[derive(Clone, Debug, Default)]
pub struct Schedule {
    pub order: Vec<NodeId>,
}

pub fn topological_dirty(graph: &Graph) -> Result<Schedule> {
    let dirty: Vec<NodeId> = graph
        .live_ids()
        .into_iter()
        .filter(|id| {
            graph
                .node(*id)
                .map(|n| n.status.needs_compute())
                .unwrap_or(false)
        })
        .collect();

    if dirty.len() > graph.limits().max_schedule_nodes {
        return Err(Error::Limit(format!(
            "schedule size {} exceeds {}",
            dirty.len(),
            graph.limits().max_schedule_nodes
        )));
    }

    let dirty_set: std::collections::BTreeSet<NodeId> = dirty.iter().copied().collect();
    let mut remaining: std::collections::BTreeSet<NodeId> = dirty_set.clone();
    let mut indeg: std::collections::BTreeMap<NodeId, usize> = std::collections::BTreeMap::new();

    for id in &dirty_set {
        let node = graph.node(*id)?;
        let deg = node
            .dependencies
            .iter()
            .filter(|d| dirty_set.contains(d))
            .count();
        indeg.insert(*id, deg);
    }

    let mut order = Vec::with_capacity(dirty_set.len());
    while !remaining.is_empty() {
        let ready: Vec<NodeId> = remaining
            .iter()
            .copied()
            .filter(|id| indeg.get(id).copied().unwrap_or(0) == 0)
            .collect();
        if ready.is_empty() {
            return Err(Error::Cycle(
                "dirty subgraph is cyclic (should have been rejected at insert)".into(),
            ));
        }
        // Deterministic: NodeId's Ord is (index, generation).
        let next = ready[0];
        remaining.remove(&next);
        order.push(next);
        let node = graph.node(next)?;
        for depent in &node.dependents {
            if let Some(d) = indeg.get_mut(depent) {
                *d = d.saturating_sub(1);
            }
        }
    }

    Ok(Schedule { order })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;

    #[test]
    fn orders_dependencies_first() {
        let mut g = Graph::new();
        let a = g.insert("a", "input").unwrap();
        let b = g.insert("b", "lex").unwrap();
        let c = g.insert("c", "parse").unwrap();
        g.add_dependency(b, a).unwrap();
        g.add_dependency(c, b).unwrap();
        g.node_mut(a).unwrap().status = delta_core::NodeStatus::Dirty;
        g.node_mut(b).unwrap().status = delta_core::NodeStatus::Dirty;
        g.node_mut(c).unwrap().status = delta_core::NodeStatus::Dirty;
        let s = topological_dirty(&g).unwrap();
        assert_eq!(s.order, vec![a, b, c]);
    }
}
