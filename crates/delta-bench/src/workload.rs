use delta_core::Delta;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workload {
    pub name: String,
    pub kind: String,
    pub size_bytes: usize,
    pub seed: u64,
    pub initial: String,
    pub deltas: Vec<Delta>,
}

#[derive(Clone, Copy, Debug)]
pub enum WorkloadKind {
    TextSmoke,
    SingleCharacter,
    SingleToken,
    SingleLine,
    FunctionBody,
    GlobalSymbol,
    LargeInsert,
    LargeDelete,
    RandomEdit,
    MassiveFanout,
    HugePaste,
}

impl WorkloadKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextSmoke => "text-smoke",
            Self::SingleCharacter => "single_character",
            Self::SingleToken => "single_token",
            Self::SingleLine => "single_line",
            Self::FunctionBody => "function_body",
            Self::GlobalSymbol => "global_symbol",
            Self::LargeInsert => "large_insert",
            Self::LargeDelete => "large_delete",
            Self::RandomEdit => "random_edit",
            Self::MassiveFanout => "massive_fanout",
            Self::HugePaste => "huge_paste",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "text-smoke" => Self::TextSmoke,
            "single_character" => Self::SingleCharacter,
            "single_token" => Self::SingleToken,
            "single_line" => Self::SingleLine,
            "function_body" => Self::FunctionBody,
            "global_symbol" => Self::GlobalSymbol,
            "large_insert" => Self::LargeInsert,
            "large_delete" => Self::LargeDelete,
            "random_edit" => Self::RandomEdit,
            "massive_fanout" => Self::MassiveFanout,
            "huge_paste" => Self::HugePaste,
            _ => return None,
        })
    }
}

pub fn workload_names() -> &'static [&'static str] {
    &[
        "text-smoke",
        "single_character",
        "single_token",
        "single_line",
        "function_body",
        "global_symbol",
        "large_insert",
        "large_delete",
        "random_edit",
        "massive_fanout",
        "huge_paste",
    ]
}

pub fn generate(kind: WorkloadKind, size_bytes: usize, seed: u64) -> Workload {
    let size = size_bytes.max(32);
    match kind {
        WorkloadKind::TextSmoke => {
            let initial = repeat_lets(2_048);
            let mid = initial.len() / 2;
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas: vec![Delta::insert(mid, "x")],
                initial,
            }
        }
        WorkloadKind::SingleCharacter => {
            let initial = repeat_lets(size);
            let at = pick(seed, 8, initial.len().saturating_sub(1));
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas: vec![Delta::insert(at, "x")],
                initial,
            }
        }
        WorkloadKind::SingleToken => {
            let initial = repeat_lets(size);
            let at = initial.find("v10").unwrap_or(initial.len() / 2);
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas: vec![Delta::insert(at + 1, "zz")],
                initial,
            }
        }
        WorkloadKind::SingleLine => {
            let initial = repeat_lets(size);
            let line = "let extra = 1;\n";
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas: vec![Delta::insert(initial.len() / 2, line)],
                initial,
            }
        }
        WorkloadKind::FunctionBody => {
            let initial = format!("fn foo() {{\n{}\n}}\n", repeat_lets(size));
            let at = initial.find('{').unwrap() + 2;
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas: vec![Delta::insert(at, "let inner = 1;\n")],
                initial,
            }
        }
        WorkloadKind::GlobalSymbol | WorkloadKind::MassiveFanout => {
            let mut initial = String::from("fn foo() {}\n");
            while initial.len() < size {
                initial.push_str("foo();\n");
            }
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas: vec![Delta::replace(3, 6, "bar")],
                initial,
            }
        }
        WorkloadKind::LargeInsert | WorkloadKind::HugePaste => {
            let initial = repeat_lets(size);
            let paste: String = "let pasted = 1;\n".repeat(200);
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas: vec![Delta::insert(initial.len() / 2, paste)],
                initial,
            }
        }
        WorkloadKind::LargeDelete => {
            let initial = repeat_lets(size);
            let start = initial.len() / 4;
            let end = (start + initial.len() / 2).min(initial.len());
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas: vec![Delta::delete(start, end)],
                initial,
            }
        }
        WorkloadKind::RandomEdit => {
            let initial = repeat_lets(size);
            let mut deltas = Vec::new();
            let mut rng = seed;
            let mut cursor_len = initial.len();
            #[allow(clippy::explicit_counter_loop)]
            for _ in 0..8 {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let at = pick(rng, 0, cursor_len.max(1));
                deltas.push(Delta::insert(at, "z"));
                cursor_len += 1;
            }
            Workload {
                name: kind.as_str().into(),
                kind: kind.as_str().into(),
                size_bytes: initial.len(),
                seed,
                deltas,
                initial,
            }
        }
    }
}

pub fn edit_at(text: &str, kind: &str) -> Delta {
    match kind {
        "begin" => Delta::insert(0, "x"),
        "end" => Delta::insert(text.len(), "x"),
        _ => Delta::insert(text.len() / 2, "x"),
    }
}

fn repeat_lets(size: usize) -> String {
    let mut out = String::with_capacity(size);
    let mut i = 0u32;
    while out.len() < size {
        out.push_str(&format!("let v{i} = {i};\n"));
        i += 1;
    }
    out
}

fn pick(seed: u64, lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    lo + (seed as usize % (hi - lo))
}
