use crate::id::NodeId;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("invalid delta: {0}")]
    InvalidDelta(String),
    #[error("unknown node {0}")]
    UnknownNode(NodeId),
    #[error("stale generation for node {0}")]
    StaleNode(NodeId),
    #[error("dependency cycle: {0}")]
    Cycle(String),
    #[error("graph error: {0}")]
    Graph(String),
    #[error("computation failed: {0}")]
    Computation(String),
    #[error("type mismatch on node {0}")]
    TypeMismatch(NodeId),
    #[error("limit exceeded: {0}")]
    Limit(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("io: {0}")]
    Io(String),
    #[error("serialize: {0}")]
    Serialize(String),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}
