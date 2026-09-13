use crate::error::{Error, Result};
use crate::range::ByteRange;
use serde::{Deserialize, Serialize};

/// A state transformation: `old + delta = new`.
///
/// Text operations are the first concrete implementations.
/// `Custom` exists so future non-text domains do not have to fork the type.
/// `Move` is reserved and currently returns [`Error::Unsupported`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Delta {
    Insert {
        offset: usize,
        text: String,
    },
    Delete {
        start: usize,
        end: usize,
    },
    Replace {
        start: usize,
        end: usize,
        text: String,
    },
    Move {
        from_start: usize,
        from_end: usize,
        to: usize,
    },
    Composite {
        ops: Vec<Delta>,
    },
    Custom {
        domain: String,
        payload: String,
    },
}

impl Delta {
    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self::Insert {
            offset,
            text: text.into(),
        }
    }

    pub fn delete(start: usize, end: usize) -> Self {
        Self::Delete { start, end }
    }

    pub fn replace(start: usize, end: usize, text: impl Into<String>) -> Self {
        Self::Replace {
            start,
            end,
            text: text.into(),
        }
    }

    pub fn composite(ops: Vec<Delta>) -> Self {
        Self::Composite { ops }
    }

    pub fn delete_range(range: ByteRange) -> Self {
        Self::Delete {
            start: range.start,
            end: range.end,
        }
    }

    /// Sequential composition: each op is in coordinates *after* the previous op.
    pub fn then(self, next: Delta) -> Self {
        match self {
            Self::Composite { mut ops } => {
                ops.push(next);
                Self::Composite { ops }
            }
            other => Self::Composite {
                ops: vec![other, next],
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Insert { text, .. } => text.is_empty(),
            Self::Delete { start, end } => start == end,
            Self::Replace { start, end, text } => start == end && text.is_empty(),
            Self::Composite { ops } => ops.is_empty() || ops.iter().all(Self::is_empty),
            Self::Move {
                from_start,
                from_end,
                ..
            } => from_start == from_end,
            Self::Custom { payload, .. } => payload.is_empty(),
        }
    }

    pub fn net_char_delta(&self) -> (isize, isize) {
        match self {
            Self::Insert { text, .. } => (text.chars().count() as isize, 0),
            Self::Delete { start, end } => (0, (*end as isize - *start as isize).max(0)),
            Self::Replace { start, end, text } => (
                text.chars().count() as isize,
                (*end as isize - *start as isize).max(0),
            ),
            Self::Composite { ops } => ops.iter().fold((0, 0), |acc, op| {
                let (i, d) = op.net_char_delta();
                (acc.0 + i, acc.1 + d)
            }),
            Self::Move { .. } | Self::Custom { .. } => (0, 0),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyStats {
    pub inserted_bytes: usize,
    pub deleted_bytes: usize,
    pub bytes_processed: usize,
    pub ops: usize,
}

/// Apply a delta, allocating a new string (full-rebuild style).
pub fn apply_delta(input: &str, delta: &Delta) -> Result<(String, ApplyStats)> {
    let mut out = input.to_owned();
    let stats = apply_delta_mut(&mut out, delta)?;
    Ok((out, stats))
}

/// Apply a delta in place. The tail still moves; we count those bytes.
pub fn apply_delta_mut(buf: &mut String, delta: &Delta) -> Result<ApplyStats> {
    let mut stats = ApplyStats::default();
    apply_inner(buf, delta, &mut stats)?;
    Ok(stats)
}

fn apply_inner(buf: &mut String, delta: &Delta, stats: &mut ApplyStats) -> Result<()> {
    match delta {
        Delta::Insert { offset, text } => {
            validate_offset(buf, *offset)?;
            let tail = buf.len().saturating_sub(*offset);
            buf.insert_str(*offset, text);
            stats.inserted_bytes += text.len();
            stats.bytes_processed += tail + text.len();
            stats.ops += 1;
            Ok(())
        }
        Delta::Delete { start, end } => {
            let range = ByteRange::new(*start, *end)?;
            range.validate_against(buf.len())?;
            validate_offset(buf, *start)?;
            validate_offset(buf, *end)?;
            let deleted = *end - *start;
            let tail = buf.len().saturating_sub(*end);
            buf.replace_range(*start..*end, "");
            stats.deleted_bytes += deleted;
            stats.bytes_processed += tail + deleted;
            stats.ops += 1;
            Ok(())
        }
        Delta::Replace { start, end, text } => {
            let range = ByteRange::new(*start, *end)?;
            range.validate_against(buf.len())?;
            validate_offset(buf, *start)?;
            validate_offset(buf, *end)?;
            let deleted = *end - *start;
            let tail = buf.len().saturating_sub(*end);
            buf.replace_range(*start..*end, text);
            stats.inserted_bytes += text.len();
            stats.deleted_bytes += deleted;
            stats.bytes_processed += tail + deleted + text.len();
            stats.ops += 1;
            Ok(())
        }
        Delta::Move { .. } => Err(Error::Unsupported(
            "Move is reserved and not applied in this preview".into(),
        )),
        Delta::Custom { domain, .. } => Err(Error::Unsupported(format!(
            "custom delta domain '{domain}' has no core applicator"
        ))),
        Delta::Composite { ops } => {
            for op in ops {
                apply_inner(buf, op, stats)?;
            }
            Ok(())
        }
    }
}

fn validate_offset(buf: &str, offset: usize) -> Result<()> {
    if offset > buf.len() {
        return Err(Error::InvalidDelta(format!(
            "offset {offset} exceeds length {}",
            buf.len()
        )));
    }
    if !buf.is_char_boundary(offset) {
        return Err(Error::InvalidDelta(format!(
            "offset {offset} is not a UTF-8 char boundary"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_delete_replace() {
        let mut s = String::from("hello");
        apply_delta_mut(&mut s, &Delta::insert(5, "!")).unwrap();
        assert_eq!(s, "hello!");
        apply_delta_mut(&mut s, &Delta::delete(4, 5)).unwrap();
        assert_eq!(s, "hell!");
        apply_delta_mut(&mut s, &Delta::replace(0, 4, "hi")).unwrap();
        assert_eq!(s, "hi!");
    }

    #[test]
    fn composite_is_sequential() {
        let (out, _) = apply_delta(
            "abcd",
            &Delta::composite(vec![Delta::insert(2, "X"), Delta::delete(0, 1)]),
        )
        .unwrap();
        // insert at 2 → "abXcd", then delete [0,1) → "bXcd"
        assert_eq!(out, "bXcd");
    }

    #[test]
    fn rejects_bad_offset() {
        let err = apply_delta("ab", &Delta::insert(3, "x")).unwrap_err();
        assert!(matches!(err, Error::InvalidDelta(_)));
    }

    #[test]
    fn rejects_mid_codepoint() {
        let s = "héllo";
        let pos = s.find('é').unwrap() + 1;
        let err = apply_delta(s, &Delta::insert(pos, "x")).unwrap_err();
        assert!(matches!(err, Error::InvalidDelta(_)));
    }

    #[test]
    fn move_is_unsupported() {
        let err = apply_delta(
            "abc",
            &Delta::Move {
                from_start: 0,
                from_end: 1,
                to: 2,
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::Unsupported(_)));
    }

    #[test]
    fn serde_roundtrip() {
        let d = Delta::replace(0, 1, "z");
        let json = serde_json::to_string(&d).unwrap();
        let back: Delta = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }
}
