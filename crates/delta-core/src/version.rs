use serde::{Deserialize, Serialize};
use std::fmt;

/// Monotonic per-node version.
///
/// Incremented when a node successfully produces a new output.
/// `0` means "never computed".
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Version(u64);

impl Version {
    pub const NEVER: Self = Self(0);

    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn bump(self) -> Self {
        Self(self.0.saturating_add(1).max(1))
    }

    pub fn is_stale_wrt(self, observed: Version) -> bool {
        self < observed
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Version({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_and_stale() {
        let v = Version::NEVER.bump();
        assert_eq!(v.get(), 1);
        assert!(Version::NEVER.is_stale_wrt(v));
        assert!(!v.is_stale_wrt(v));
    }
}
