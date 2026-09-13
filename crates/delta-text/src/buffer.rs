use delta_core::{apply_delta, apply_delta_mut, ApplyStats, Delta, Result};
use serde::{Deserialize, Serialize};

/// Measured text container. Incremental apply is `String` splice — not a rope.
///
/// We count tail bytes moved. That is the honest cost of this representation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TextBuffer {
    pub text: String,
}

impl TextBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn apply(&mut self, delta: &Delta) -> Result<ApplyStats> {
        apply_delta_mut(&mut self.text, delta)
    }

    /// Oracle: rebuild a brand-new string.
    pub fn apply_full(text: &str, delta: &Delta) -> Result<(String, ApplyStats)> {
        apply_delta(text, delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_matches_full() {
        let mut buf = TextBuffer::new("hello world");
        let d = Delta::replace(6, 11, "delta");
        let (full, _) = TextBuffer::apply_full(&buf.text, &d).unwrap();
        buf.apply(&d).unwrap();
        assert_eq!(buf.text, full);
    }
}
