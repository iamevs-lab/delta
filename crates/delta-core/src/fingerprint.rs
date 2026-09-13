use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Deterministic 64-bit fingerprint (FNV-1a).
///
/// Stable across platforms and Rust versions for the same bytes.
/// This is not a cryptographic hash. Collisions are possible.
///
/// Hashing has a cost. Callers must record [`FingerprintTimer`] if they
/// want to study when proving "unchanged" exceeds recomputing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Fingerprint(u64);

impl Fingerprint {
    pub const fn from_raw(v: u64) -> Self {
        Self(v)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(fnv1a64(bytes))
    }

    pub fn of_u64(v: u64) -> Self {
        Self(fnv1a64(&v.to_le_bytes()))
    }

    pub fn combine(self, other: Self) -> Self {
        let mut buf = [0u8; 16];
        buf[..8].copy_from_slice(&self.0.to_le_bytes());
        buf[8..].copy_from_slice(&other.0.to_le_bytes());
        Self(fnv1a64(&buf))
    }
}

impl std::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fp({:016x})", self.0)
    }
}

impl std::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for b in data {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub struct FingerprintTimer {
    start: Instant,
}

impl FingerprintTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn finish(self, bytes: &[u8]) -> (Fingerprint, Duration) {
        let fp = Fingerprint::of_bytes(bytes);
        (fp, self.start.elapsed())
    }
}

pub fn fingerprint_bytes(bytes: &[u8]) -> (Fingerprint, Duration) {
    FingerprintTimer::start().finish(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_sensitive() {
        let a = Fingerprint::of_bytes(b"hello");
        let b = Fingerprint::of_bytes(b"hello");
        let c = Fingerprint::of_bytes(b"hallo");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn empty_is_offset_basis() {
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
    }
}
