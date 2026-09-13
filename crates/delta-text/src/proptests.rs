use crate::pipeline::LanguagePipeline;
use delta_core::Delta;
use proptest::prelude::*;

fn arb_program(stmts: usize) -> String {
    (0..stmts).map(|i| format!("let v{i} = {i};\n")).collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn incremental_matches_full_random_edits(
        stmt_count in 2usize..12,
        edits in prop::collection::vec((0usize..40, 0usize..4, "[a-z]{0,3}"), 1..5)
    ) {
        let src = arb_program(stmt_count);
        let mut inc = LanguagePipeline::from_text(&src);
        let mut full = LanguagePipeline::from_text(&src);
        for (x, y, t) in edits {
            let len = inc.source.len();
            if len == 0 {
                continue;
            }
            let start = floor_boundary(&inc.source, x.min(len));
            let end = floor_boundary(&inc.source, (start + y).min(len));
            if start > end {
                continue;
            }
            let delta = match x % 3 {
                0 => Delta::insert(start, t.clone()),
                1 if start < end => Delta::delete(start, end),
                _ => Delta::replace(start, end, t.clone()),
            };
            if inc.apply_incremental(&delta).is_err() {
                continue;
            }
            full.apply_full(&delta).unwrap();
            prop_assert_eq!(&inc.source, &full.source);
            prop_assert!(inc.tokens.semantic_eq(&full.tokens));
            prop_assert!(inc.program.semantic_eq(&full.program));
            prop_assert!(inc.symbols.semantic_eq(&full.symbols));
            prop_assert!(inc.diagnostics.semantic_eq(&full.diagnostics));
        }
    }
}

fn floor_boundary(s: &str, mut i: usize) -> usize {
    if i > s.len() {
        i = s.len();
    }
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
