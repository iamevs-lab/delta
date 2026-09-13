//! Property tests: in-place apply and copy-apply must agree, and repeated
//! application is deterministic.

use crate::delta::{apply_delta, apply_delta_mut, Delta};
use proptest::prelude::*;

fn arb_text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(b"abcdef \n".to_vec()), 0..64)
        .prop_map(|v| String::from_utf8(v).unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn apply_is_deterministic(
        text in arb_text(),
        ops in prop::collection::vec((0usize..32, 0usize..32, "[a-z]{0,5}"), 1..6)
    ) {
        let mut a = text.clone();
        let mut b = text;
        for (x, y, t) in ops {
            let len = a.len();
            let start = if len == 0 { 0 } else { x.min(len) };
            let end = (start + y).min(len);
            let delta = match x % 3 {
                0 => Delta::insert(start, t.clone()),
                1 => Delta::delete(start, end),
                _ => Delta::replace(start, end, t.clone()),
            };
            let (copied, _) = apply_delta(&a, &delta).unwrap();
            apply_delta_mut(&mut a, &delta).unwrap();
            apply_delta_mut(&mut b, &delta).unwrap();
            prop_assert_eq!(&a, &copied);
            prop_assert_eq!(&a, &b);
        }
    }
}
