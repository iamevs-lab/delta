use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Half-open byte range `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl ByteRange {
    pub fn new(start: usize, end: usize) -> Result<Self> {
        if start > end {
            return Err(Error::InvalidDelta(format!(
                "range start {start} > end {end}"
            )));
        }
        Ok(Self { start, end })
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn contains(self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }

    pub fn validate_against(self, len: usize) -> Result<()> {
        if self.end > len {
            return Err(Error::InvalidDelta(format!(
                "range [{}, {}) exceeds length {len}",
                self.start, self.end
            )));
        }
        Ok(())
    }
}
