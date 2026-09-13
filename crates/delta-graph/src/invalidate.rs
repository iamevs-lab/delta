use crate::graph::Graph;
use delta_core::{Error, InvalidationReason, NodeId, NodeStatus, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationMode {
    /// Walk dependents now. Default for the laboratory.
    #[default]
    Eager,
    /// Mark only the seed. Dependents discover staleness on query.
    Lazy,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InvalidationReport {
    pub seed: Option<String>,
    pub invalidated: Vec<NodeId>,
    pub fanout: usize,
    pub mode: String,
}

pub fn invalidate(
    graph: &mut Graph,
    seed: NodeId,
    include_seed: bool,
    mode: InvalidationMode,
    reason: InvalidationReason,
) -> Result<InvalidationReport> {
    let mut report = InvalidationReport {
        seed: graph.node(seed).ok().map(|n| n.name.clone()),
        mode: format!("{mode:?}"),
        ..Default::default()
    };

    match mode {
        InvalidationMode::Lazy => {
            if include_seed {
                mark(graph, seed, reason)?;
                report.invalidated.push(seed);
            }
            report.fanout = report.invalidated.len();
            Ok(report)
        }
        InvalidationMode::Eager => {
            let mut work: Vec<NodeId> = Vec::new();
            if include_seed {
                work.push(seed);
            } else {
                work.extend(graph.node(seed)?.dependents.iter().copied());
            }

            let mut seen = std::collections::BTreeSet::new();
            let limit = graph.limits().max_invalidation_nodes;

            while let Some(id) = work.pop() {
                if !seen.insert(id) {
                    continue;
                }
                if seen.len() > limit {
                    return Err(Error::Limit(format!("invalidation exceeded {limit} nodes")));
                }
                mark(graph, id, reason.clone())?;
                report.invalidated.push(id);
                let dependents = graph.node(id)?.dependents.clone();
                work.extend(dependents);
            }
            report.fanout = report.invalidated.len();
            Ok(report)
        }
    }
}

fn mark(graph: &mut Graph, id: NodeId, reason: InvalidationReason) -> Result<()> {
    let node = graph.node_mut(id)?;
    if node.status != NodeStatus::Computing {
        node.status = NodeStatus::Dirty;
    }
    node.last_invalidation = Some(reason);
    Ok(())
}
