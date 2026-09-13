use crate::compute::{Computation, ComputeCtx, ComputeMode, ComputeOutput};
use crate::log::LogLevel;
use crate::policy::{RecomputeDecision, RecomputePolicy};
use crate::report::{ChangeSummary, UpdateReport};
use delta_core::{
    apply_delta_mut, Delta, DurationNanos, Error, Fingerprint, InvalidationReason, NodeId,
    NodeStatus, Result, Version, WorkMetrics,
};
use delta_graph::{Graph, InvalidationMode, NodeInspect};
use delta_store::{MemoryStore, Store};
use std::any::Any;
use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub policy: RecomputePolicy,
    pub invalidation: InvalidationMode,
    pub log_level: LogLevel,
    /// If true, skip recompute when the combined input fingerprint matches.
    /// Off by default — hashing is a cost we want to measure, not assume.
    pub fingerprint_inputs: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            policy: RecomputePolicy::AlwaysIncremental,
            invalidation: InvalidationMode::Eager,
            log_level: LogLevel::Normal,
            fingerprint_inputs: false,
        }
    }
}

pub struct Engine {
    graph: Graph,
    store: MemoryStore,
    computations: HashMap<NodeId, Box<dyn Computation>>,
    inputs: BTreeSet<NodeId>,
    config: EngineConfig,
    last_delta: Option<(NodeId, Delta)>,
    last_report: UpdateReport,
}

impl Engine {
    pub fn new() -> Self {
        Self::with_config(EngineConfig::default())
    }

    pub fn with_config(config: EngineConfig) -> Self {
        Self {
            graph: Graph::new(),
            store: MemoryStore::new(),
            computations: HashMap::new(),
            inputs: BTreeSet::new(),
            config,
            last_delta: None,
            last_report: UpdateReport::default(),
        }
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn set_policy(&mut self, policy: RecomputePolicy) {
        self.config.policy = policy;
    }

    pub fn set_invalidation(&mut self, mode: InvalidationMode) {
        self.config.invalidation = mode;
    }

    pub fn set_log_level(&mut self, level: LogLevel) {
        self.config.log_level = level;
    }

    pub fn last_report(&self) -> &UpdateReport {
        &self.last_report
    }

    pub fn register_input(
        &mut self,
        name: impl Into<String>,
        kind: impl Into<String>,
        value: Box<dyn Any + Send + Sync>,
    ) -> Result<NodeId> {
        let id = self.graph.insert(name, kind)?;
        self.store.insert(id, value);
        self.inputs.insert(id);
        let node = self.graph.node_mut(id)?;
        node.status = NodeStatus::Clean;
        node.version = Version::NEVER.bump();
        Ok(id)
    }

    pub fn register(
        &mut self,
        computation: Box<dyn Computation>,
        deps: &[NodeId],
    ) -> Result<NodeId> {
        let id = self.graph.insert(computation.name(), computation.kind())?;
        for dep in deps {
            self.graph.add_dependency(id, *dep)?;
        }
        self.computations.insert(id, computation);
        Ok(id)
    }

    pub fn get<T: 'static>(&self, id: NodeId) -> Result<&T> {
        self.store
            .get(id)
            .and_then(|v| v.downcast_ref::<T>())
            .ok_or(Error::TypeMismatch(id))
    }

    pub fn inspect(&self, id: NodeId) -> Result<NodeInspect> {
        self.graph.inspect(id)
    }

    pub fn set_input<T: Send + Sync + 'static>(
        &mut self,
        id: NodeId,
        value: T,
    ) -> Result<UpdateReport> {
        if !self.inputs.contains(&id) {
            return Err(Error::Graph(format!("{id} is not an input node")));
        }
        self.store.insert(id, Box::new(value));
        self.bump_input(id)?;
        let inv_start = Instant::now();
        let inv = self.graph.invalidate(
            id,
            false,
            self.config.invalidation,
            InvalidationReason::InputChanged,
        )?;
        let inv_nanos = DurationNanos::from_duration(inv_start.elapsed());
        self.last_delta = None;
        let mut report = self.recompute()?;
        report.invalidation_nanos = inv_nanos;
        report.invalidation_fanout = inv.fanout;
        report.invalidated_nodes = inv.invalidated.len();
        report.finalize_percents();
        self.last_report = report.clone();
        Ok(report)
    }

    pub fn apply_delta(&mut self, id: NodeId, delta: Delta) -> Result<UpdateReport> {
        if !self.inputs.contains(&id) {
            return Err(Error::Graph(format!("{id} is not an input node")));
        }
        let mut buf = self.get::<String>(id)?.clone();
        let stats = apply_delta_mut(&mut buf, &delta)?;
        let (ins, del) = delta.net_char_delta();
        self.store.insert(id, Box::new(buf));
        self.bump_input(id)?;

        let inv_start = Instant::now();
        let inv = self.graph.invalidate(
            id,
            false,
            self.config.invalidation,
            InvalidationReason::InputChanged,
        )?;
        let inv_nanos = DurationNanos::from_duration(inv_start.elapsed());
        self.last_delta = Some((id, delta));

        let mut report = self.recompute()?;
        report.invalidation_nanos = inv_nanos;
        report.invalidation_fanout = inv.fanout;
        report.invalidated_nodes = inv.invalidated.len();
        report.change = Some(ChangeSummary {
            inserted_chars: ins as i64,
            deleted_chars: del as i64,
            inserted_bytes: stats.inserted_bytes,
            deleted_bytes: stats.deleted_bytes,
        });
        report.work.bytes_processed += stats.bytes_processed;
        report.finalize_percents();
        self.last_report = report.clone();
        Ok(report)
    }

    /// Recompute dirty nodes without applying a new delta.
    pub fn update(&mut self) -> Result<UpdateReport> {
        let mut report = self.recompute()?;
        report.finalize_percents();
        self.last_report = report.clone();
        Ok(report)
    }

    pub fn reset(&mut self) {
        self.graph = Graph::new();
        self.store.clear();
        self.computations.clear();
        self.inputs.clear();
        self.last_delta = None;
        self.last_report = UpdateReport::default();
    }

    fn bump_input(&mut self, id: NodeId) -> Result<()> {
        let node = self.graph.node_mut(id)?;
        node.status = NodeStatus::Clean;
        node.version = node.version.bump();
        node.last_invalidation = Some(InvalidationReason::InputChanged);
        Ok(())
    }

    fn discover_lazy(&mut self) -> Result<()> {
        if self.config.invalidation != InvalidationMode::Lazy {
            return Ok(());
        }
        let Some((src, _)) = &self.last_delta else {
            return Ok(());
        };
        let src = *src;
        self.graph.invalidate(
            src,
            false,
            InvalidationMode::Eager,
            InvalidationReason::DependencyChanged {
                dependency: self.graph.node(src)?.name.clone(),
            },
        )?;
        Ok(())
    }

    fn recompute(&mut self) -> Result<UpdateReport> {
        let started = Instant::now();
        self.discover_lazy()?;

        let stats_before = self.graph.stats();
        let dirty =
            stats_before.dirty_nodes + stats_before.pending_nodes + stats_before.failed_nodes;
        let decision = self.config.policy.decide(dirty, stats_before.total_nodes);
        let (est_inc, est_full) = self
            .config
            .policy
            .estimate_costs(dirty, stats_before.total_nodes);

        let mut log = Vec::new();
        self.note(
            &mut log,
            LogLevel::Normal,
            format!("decision={}", decision.label()),
        );

        if decision.is_full() {
            self.mark_all_computed_dirty()?;
        }

        let schedule = self.graph.schedule_dirty()?;
        self.note(
            &mut log,
            LogLevel::Verbose,
            format!("schedule {} nodes", schedule.order.len()),
        );

        let mut work = WorkMetrics::default();
        let mut hashing = DurationNanos(0);
        let mut recomputed = 0usize;
        let mut reused_skip = 0usize;
        let mut failed = 0usize;

        let order = schedule.order.clone();
        for id in order {
            match self.run_one(id, &decision, &mut hashing, &mut log)? {
                NodeOutcome::Recomputed(w) => {
                    recomputed += 1;
                    work.merge(&w);
                }
                NodeOutcome::Reused => reused_skip += 1,
                NodeOutcome::Failed => failed += 1,
            }
        }

        let stats = self.graph.stats();
        let reused = stats
            .clean_nodes
            .saturating_sub(recomputed.min(stats.clean_nodes));
        // clean nodes include inputs + reused compute nodes. Add fingerprint skips.
        let reused = reused + reused_skip;

        let mut report = UpdateReport {
            decision,
            estimated_incremental_cost: est_inc,
            estimated_full_cost: est_full,
            invalidated_nodes: dirty,
            recomputed_nodes: recomputed,
            reused_nodes: reused,
            dirty_nodes: stats.dirty_nodes,
            total_nodes: stats.total_nodes,
            failed_nodes: failed,
            dependency_edges: stats.dependency_edges,
            invalidation_fanout: dirty,
            max_dependency_depth: stats.max_dependency_depth,
            execution_nanos: DurationNanos::from_duration(started.elapsed()),
            hashing_nanos: hashing,
            work,
            log,
            ..UpdateReport::default()
        };
        report.finalize_percents();
        Ok(report)
    }

    fn run_one(
        &mut self,
        id: NodeId,
        decision: &RecomputeDecision,
        hashing: &mut DurationNanos,
        log: &mut Vec<String>,
    ) -> Result<NodeOutcome> {
        let deps = self.graph.node(id)?.dependencies.clone();
        let name = self.graph.node(id)?.name.clone();

        if self.config.fingerprint_inputs {
            if let Some(outcome) = self.try_fingerprint_reuse(id, &deps, hashing)? {
                self.note(log, LogLevel::Debug, format!("{name}: fingerprint reuse"));
                return Ok(outcome);
            }
        }

        let mode = if decision.is_full() {
            ComputeMode::Full
        } else {
            ComputeMode::Incremental
        };
        let delta = self.last_delta.as_ref().and_then(|(src, d)| {
            if deps.contains(src) {
                Some(d.clone())
            } else {
                None
            }
        });

        {
            let node = self.graph.node_mut(id)?;
            node.status = NodeStatus::Computing;
        }

        let t0 = Instant::now();
        let computed = self.invoke(id, &deps, mode, delta.as_ref());
        let elapsed = t0.elapsed();

        match computed {
            Ok(output) => {
                let ComputeOutput {
                    value,
                    fingerprint,
                    work,
                } = output;
                self.store.insert(id, value);
                let node = self.graph.node_mut(id)?;
                node.status = NodeStatus::Clean;
                node.version = node.version.bump();
                node.output_fingerprint = fingerprint;
                node.last_compute_nanos = elapsed.as_nanos();
                node.last_recompute_count = node.last_recompute_count.saturating_add(1);
                hashing.0 += work.hashing_nanos.0;
                self.note(
                    log,
                    LogLevel::Debug,
                    format!(
                        "{name}: recomputed in {:.4} ms",
                        elapsed.as_secs_f64() * 1000.0
                    ),
                );
                Ok(NodeOutcome::Recomputed(work))
            }
            Err(err) => {
                let node = self.graph.node_mut(id)?;
                node.status = NodeStatus::Failed;
                node.last_compute_nanos = elapsed.as_nanos();
                self.note(log, LogLevel::Normal, format!("{name}: {err}"));
                Ok(NodeOutcome::Failed)
            }
        }
    }

    fn invoke(
        &self,
        id: NodeId,
        deps: &[NodeId],
        mode: ComputeMode,
        delta: Option<&Delta>,
    ) -> Result<ComputeOutput> {
        let comp = self
            .computations
            .get(&id)
            .ok_or_else(|| Error::Graph(format!("{id} has no computation")))?;
        let previous = self.store.get(id);
        let ctx = ComputeCtx {
            node: id,
            inputs: deps,
            store: &self.store,
            previous,
            delta,
            mode,
        };
        if mode == ComputeMode::Incremental {
            if let Some(out) = comp.compute_incremental(ctx)? {
                return Ok(out);
            }
            let ctx = ComputeCtx {
                node: id,
                inputs: deps,
                store: &self.store,
                previous: self.store.get(id),
                delta,
                mode: ComputeMode::Full,
            };
            return comp.compute(ctx);
        }
        comp.compute(ctx)
    }

    fn try_fingerprint_reuse(
        &mut self,
        id: NodeId,
        deps: &[NodeId],
        hashing: &mut DurationNanos,
    ) -> Result<Option<NodeOutcome>> {
        let t0 = Instant::now();
        let mut acc = Fingerprint::of_bytes(b"in");
        for dep in deps {
            let Some(fp) = self.graph.node(*dep)?.output_fingerprint else {
                return Ok(None);
            };
            acc = acc.combine(fp);
        }
        hashing.0 += t0.elapsed().as_nanos();
        let node = self.graph.node(id)?;
        if node.input_fingerprint == Some(acc) && self.store.contains(id) && node.version.get() > 0
        {
            let node = self.graph.node_mut(id)?;
            node.status = NodeStatus::Clean;
            node.input_fingerprint = Some(acc);
            return Ok(Some(NodeOutcome::Reused));
        }
        self.graph.node_mut(id)?.input_fingerprint = Some(acc);
        Ok(None)
    }

    fn mark_all_computed_dirty(&mut self) -> Result<()> {
        let ids: Vec<NodeId> = self
            .graph
            .live_ids()
            .into_iter()
            .filter(|id| !self.inputs.contains(id))
            .collect();
        for id in ids {
            let node = self.graph.node_mut(id)?;
            node.status = NodeStatus::Dirty;
            node.last_invalidation = Some(InvalidationReason::PolicyChoseFull);
        }
        Ok(())
    }

    fn note(&self, log: &mut Vec<String>, needed: LogLevel, msg: impl Into<String>) {
        if self.config.log_level.allows(needed) {
            log.push(msg.into());
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

enum NodeOutcome {
    Recomputed(WorkMetrics),
    Reused,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Double;

    impl Computation for Double {
        fn name(&self) -> &str {
            "double"
        }
        fn kind(&self) -> &str {
            "math"
        }
        fn compute(&self, ctx: ComputeCtx<'_>) -> Result<ComputeOutput> {
            let n: &i32 = ctx.input(0)?;
            Ok(ComputeOutput {
                value: Box::new(n * 2),
                fingerprint: None,
                work: WorkMetrics {
                    items_recomputed: 1,
                    items_total: 1,
                    ..WorkMetrics::default()
                },
            })
        }
    }

    #[test]
    fn full_and_incremental_agree() {
        let mut engine = Engine::new();
        let src = engine
            .register_input("n", "input", Box::new(3_i32))
            .unwrap();
        let out = engine.register(Box::new(Double), &[src]).unwrap();
        engine.update().unwrap();
        assert_eq!(*engine.get::<i32>(out).unwrap(), 6);
        engine.set_input(src, 5_i32).unwrap();
        assert_eq!(*engine.get::<i32>(out).unwrap(), 10);
    }
}
