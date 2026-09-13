use delta_core::{Delta, Fingerprint, NodeId, Result, WorkMetrics};
use delta_store::StoreView;
use std::any::Any;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputeMode {
    Full,
    Incremental,
}

pub struct ComputeOutput {
    pub value: Box<dyn Any + Send + Sync>,
    pub fingerprint: Option<Fingerprint>,
    pub work: WorkMetrics,
}

pub struct ComputeCtx<'a> {
    pub node: NodeId,
    pub inputs: &'a [NodeId],
    pub store: &'a dyn StoreView,
    pub previous: Option<&'a (dyn Any + Send + Sync)>,
    pub delta: Option<&'a Delta>,
    pub mode: ComputeMode,
}

impl ComputeCtx<'_> {
    pub fn input<T: 'static>(&self, index: usize) -> Result<&T> {
        let id = self
            .inputs
            .get(index)
            .ok_or_else(|| delta_core::Error::Computation(format!("missing input {index}")))?;
        self.store
            .get(*id)
            .and_then(|v| v.downcast_ref::<T>())
            .ok_or(delta_core::Error::TypeMismatch(*id))
    }

    pub fn previous_as<T: 'static>(&self) -> Option<&T> {
        self.previous.and_then(|p| p.downcast_ref::<T>())
    }
}

/// A reusable unit of work.
///
/// `compute` is the oracle.
/// `compute_incremental` may return `None` to request the oracle.
pub trait Computation: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> &str;

    fn compute(&self, ctx: ComputeCtx<'_>) -> Result<ComputeOutput>;

    fn compute_incremental(&self, ctx: ComputeCtx<'_>) -> Result<Option<ComputeOutput>> {
        let _ = ctx;
        Ok(None)
    }
}
