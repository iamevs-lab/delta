use clap::{Parser, Subcommand};
use delta_bench::{generate, load_replay, run_compare, save_replay, workload_names, WorkloadKind};
use delta_core::Delta;
use delta_graph::InvalidationMode;
use delta_runtime::{Engine, LogLevel, RecomputePolicy};
use delta_text::{install_language_graph, LanguageHandles, LanguagePipeline};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;

mod nvim_install;

#[derive(Parser)]
#[command(name = "delta-cli", about = "EVS Delta laboratory CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write a starter replay file.
    Init {
        #[arg(long, default_value = "experiments/replay.json")]
        out: PathBuf,
    },
    Inspect {
        #[arg(long)]
        replay: Option<PathBuf>,
        #[arg(long)]
        node: Option<String>,
    },
    Graph {
        #[arg(long)]
        replay: Option<PathBuf>,
    },
    Benchmark {
        #[arg(long, default_value = "text-smoke")]
        workload: String,
        #[arg(long, default_value_t = 4096)]
        size: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Replay {
        file: PathBuf,
        #[arg(long)]
        incremental: bool,
    },
    Compare {
        file: PathBuf,
    },
    Stats {
        #[arg(long)]
        replay: Option<PathBuf>,
    },
    /// JSON-lines server for Neovim and other adapters.
    Serve,
}

fn main() {
    if std::env::var_os("DELTA_SKIP_NVIM_INSTALL").is_none() {
        let _ = nvim_install::install_current_exe();
    }
    let cli = Cli::parse();
    if let Err(e) = run(cli.command) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cmd: Command) -> Result<(), String> {
    match cmd {
        Command::Init { out } => {
            let w = generate(WorkloadKind::TextSmoke, 2048, 1);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            save_replay(&out, &w).map_err(|e| e.to_string())?;
            println!("wrote {}", out.display());
            Ok(())
        }
        Command::Inspect { replay, node } => {
            let (engine, handles) = engine_from_replay(replay)?;
            if let Some(name) = node {
                for id in engine.graph().live_ids() {
                    let n = engine.graph().node(id).map_err(|e| e.to_string())?;
                    if n.name == name {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &engine.inspect(id).map_err(|e| e.to_string())?
                            )
                            .map_err(|e| e.to_string())?
                        );
                        return Ok(());
                    }
                }
                return Err(format!("no node named {name}"));
            }
            println!(
                "{}",
                serde_json::to_string_pretty(engine.last_report()).map_err(|e| e.to_string())?
            );
            let _ = handles;
            Ok(())
        }
        Command::Graph { replay } => {
            let (engine, _) = engine_from_replay(replay)?;
            print_graph(&engine);
            Ok(())
        }
        Command::Benchmark {
            workload,
            size,
            seed,
            out,
        } => {
            if workload == "list" {
                for n in workload_names() {
                    println!("{n}");
                }
                return Ok(());
            }
            let kind = WorkloadKind::parse(&workload)
                .ok_or_else(|| format!("unknown workload {workload}"))?;
            let w = generate(kind, size, seed);
            let report = run_compare(&w)?;
            let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
            println!("{json}");
            if let Some(dir) = out {
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let name = format!("{}-{}-{}.json", w.name, w.size_bytes, w.seed);
                std::fs::write(dir.join(name), json).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        Command::Replay { file, incremental } => {
            let replay = load_replay(&file).map_err(|e| e.to_string())?;
            let mut pipe = LanguagePipeline::from_text(&replay.workload.initial);
            for d in &replay.workload.deltas {
                if incremental {
                    pipe.apply_incremental(d).map_err(|e| e.to_string())?;
                } else {
                    pipe.apply_full(d).map_err(|e| e.to_string())?;
                }
            }
            println!(
                "replayed {} deltas ({}). source_len={}",
                replay.workload.deltas.len(),
                if incremental { "incremental" } else { "full" },
                pipe.source.len()
            );
            Ok(())
        }
        Command::Compare { file } => {
            let replay = load_replay(&file).map_err(|e| e.to_string())?;
            let report = run_compare(&replay.workload)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        Command::Stats { replay } => {
            let (engine, _) = engine_from_replay(replay)?;
            let s = engine.graph().stats();
            println!(
                "{}",
                serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        Command::Serve => serve(),
    }
}

fn engine_from_replay(replay: Option<PathBuf>) -> Result<(Engine, LanguageHandles), String> {
    let mut engine = Engine::new();
    engine.set_log_level(LogLevel::Normal);
    engine.set_invalidation(InvalidationMode::Eager);
    engine.set_policy(RecomputePolicy::AlwaysIncremental);
    let text = if let Some(path) = replay {
        let r = load_replay(&path).map_err(|e| e.to_string())?;
        r.workload.initial
    } else {
        "let foo = 10;\nlet bar = 20;\nlet baz = foo + bar;\n".into()
    };
    let handles = install_language_graph(&mut engine, text).map_err(|e| e.to_string())?;
    Ok((engine, handles))
}

fn print_graph(engine: &Engine) {
    println!("SOURCE");
    println!("   |");
    println!("   v");
    println!("TOKENS");
    println!("   |");
    println!("   v");
    println!("  AST");
    println!(" / | \\");
    println!("v  v  v");
    println!("SYMBOL  DIAGNOSTIC  (structure)");
    println!();
    for id in engine.graph().live_ids() {
        if let Ok(n) = engine.graph().node(id) {
            let mark = match n.status {
                delta_core::NodeStatus::Clean => "clean",
                delta_core::NodeStatus::Dirty => "dirty",
                delta_core::NodeStatus::Pending => "pending",
                delta_core::NodeStatus::Computing => "computing",
                delta_core::NodeStatus::Failed => "failed",
            };
            println!("  {}  {}  {}  status={mark}", n.id, n.name, n.version);
        }
    }
}

#[derive(Debug, Deserialize)]
struct RpcReq {
    id: u64,
    cmd: String,
    buf: Option<u64>,
    text: Option<String>,
    path: Option<String>,
    delta: Option<Delta>,
}

#[derive(Serialize)]
struct RpcRes {
    id: u64,
    ok: bool,
    error: Option<String>,
    #[serde(flatten)]
    payload: serde_json::Value,
}

struct BufferState {
    engine: Engine,
    handles: Option<LanguageHandles>,
    pipeline: LanguagePipeline,
    last_status: Option<StatusPayload>,
}

struct Session {
    buffers: HashMap<u64, BufferState>,
    last_buf: Option<u64>,
}

impl Session {
    fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            last_buf: None,
        }
    }

    fn buf_id(&self, req: &RpcReq) -> u64 {
        req.buf.or(self.last_buf).unwrap_or(1)
    }

    fn get(&self, id: u64) -> Option<&BufferState> {
        self.buffers.get(&id)
    }
}

fn serve() -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut session = Session::new();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let req: RpcReq = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                write_res(
                    &mut stdout,
                    RpcRes {
                        id: 0,
                        ok: false,
                        error: Some(e.to_string()),
                        payload: serde_json::json!({}),
                    },
                )?;
                continue;
            }
        };
        let res = handle(&mut session, req);
        write_res(&mut stdout, res)?;
    }
    Ok(())
}

fn handle(session: &mut Session, req: RpcReq) -> RpcRes {
    let id = req.id;
    let buf = session.buf_id(&req);
    let result = match req.cmd.as_str() {
        "open" => {
            let text = req.text.unwrap_or_default();
            if text.is_empty() {
                if let Some(existing) = session.buffers.get(&buf) {
                    if !existing.pipeline.source.is_empty() {
                        session.last_buf = Some(buf);
                        return RpcRes {
                            id,
                            ok: true,
                            error: None,
                            payload: serde_json::json!({
                                "opened": true,
                                "ignored_empty": true,
                                "buf": buf,
                                "source_len": existing.pipeline.source.len(),
                            }),
                        };
                    }
                }
            }
            let mut engine = Engine::new();
            engine.set_log_level(LogLevel::Normal);
            engine.set_invalidation(InvalidationMode::Eager);
            engine.set_policy(RecomputePolicy::AlwaysIncremental);
            match install_language_graph(&mut engine, text.clone()) {
                Ok(handles) => {
                    let pipeline = LanguagePipeline::from_text(&text);
                    let last_status = Some(status_payload(
                        &pipeline,
                        &engine,
                        &delta_text::PipelineSnapshot {
                            source_len: pipeline.source.len(),
                            tokens_total: pipeline.tokens.tokens.len(),
                            mode: "open".into(),
                            ..Default::default()
                        },
                    ));
                    session.buffers.insert(
                        buf,
                        BufferState {
                            engine,
                            handles: Some(handles),
                            pipeline,
                            last_status,
                        },
                    );
                    session.last_buf = Some(buf);
                    Ok(serde_json::json!({
                        "opened": true,
                        "path": req.path,
                        "buf": buf,
                        "source_len": text.len(),
                    }))
                }
                Err(e) => Err(e.to_string()),
            }
        }
        "edit" => {
            let Some(delta) = req.delta else {
                return fail(id, "missing delta");
            };
            let Some(state) = session.buffers.get_mut(&buf) else {
                return fail(id, "no buffer open");
            };
            session.last_buf = Some(buf);
            match state.pipeline.apply_incremental(&delta) {
                Ok(snap) => {
                    if let Some(h) = state.handles {
                        let _ = state.engine.apply_delta(h.source, delta);
                    }
                    let payload = status_payload(&state.pipeline, &state.engine, &snap);
                    state.last_status = Some(payload.clone());
                    Ok(serde_json::to_value(&payload).unwrap_or_else(|_| serde_json::json!({})))
                }
                Err(e) => Err(e.to_string()),
            }
        }
        "status" => {
            let Some(state) = session.get(buf) else {
                return fail(id, "no buffer open");
            };
            let payload = state.last_status.clone().unwrap_or_else(|| {
                status_payload(
                    &state.pipeline,
                    &state.engine,
                    &delta_text::PipelineSnapshot {
                        source_len: state.pipeline.source.len(),
                        tokens_total: state.pipeline.tokens.tokens.len(),
                        mode: "status".into(),
                        ..Default::default()
                    },
                )
            });
            Ok(serde_json::to_value(&payload).unwrap_or_else(|_| serde_json::json!({})))
        }
        "inspect" => {
            let Some(state) = session.get(buf) else {
                return fail(id, "no buffer open");
            };
            Ok(serde_json::json!({
                "source_len": state.pipeline.source.len(),
                "tokens": state.pipeline.tokens.tokens.len(),
                "stmts": state.pipeline.program.stmts.len(),
                "defs": state.pipeline.symbols.defs.len(),
                "refs": state.pipeline.symbols.refs.len(),
                "diagnostics": state.pipeline.diagnostics.items.len(),
            }))
        }
        "graph" => {
            let Some(state) = session.get(buf) else {
                return fail(id, "no buffer open");
            };
            Ok(serde_json::json!({
                "ascii": graph_ascii(&state.engine),
                "nodes": state.engine.graph().live_ids().iter().filter_map(|id| {
                    state.engine.inspect(*id).ok()
                }).collect::<Vec<_>>(),
            }))
        }
        "close" => {
            session.buffers.remove(&buf);
            if session.last_buf == Some(buf) {
                session.last_buf = session.buffers.keys().copied().next();
            }
            Ok(serde_json::json!({"closed": true, "buf": buf}))
        }
        "reset" => {
            if req.buf.is_some() {
                session.buffers.remove(&buf);
                if session.last_buf == Some(buf) {
                    session.last_buf = session.buffers.keys().copied().next();
                }
            } else {
                session.buffers.clear();
                session.last_buf = None;
            }
            Ok(serde_json::json!({"reset": true, "buf": buf}))
        }
        "ping" => Ok(serde_json::json!({"pong": true, "buf": buf})),
        "stats" => {
            let Some(state) = session.get(buf) else {
                return fail(id, "no buffer open");
            };
            let s = state.engine.graph().stats();
            Ok(serde_json::to_value(s).unwrap_or_else(|_| serde_json::json!({})))
        }
        other => Err(format!("unknown cmd {other}")),
    };
    match result {
        Ok(payload) => RpcRes {
            id,
            ok: true,
            error: None,
            payload,
        },
        Err(e) => fail(id, e),
    }
}

fn fail(id: u64, error: impl Into<String>) -> RpcRes {
    RpcRes {
        id,
        ok: false,
        error: Some(error.into()),
        payload: serde_json::json!({}),
    }
}

fn write_res(out: &mut impl Write, res: RpcRes) -> Result<(), String> {
    let mut line = serde_json::to_string(&res).map_err(|e| e.to_string())?;
    line.push('\n');
    out.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}

#[derive(Clone, Serialize)]
struct StatusPayload {
    inserted_chars: i64,
    deleted_chars: i64,
    tokens_reused: usize,
    tokens_recomputed: usize,
    ast_reused: usize,
    ast_recomputed: usize,
    symbols_reused: usize,
    symbols_recomputed: usize,
    diagnostics_reused: usize,
    diagnostics_recomputed: usize,
    diagnostics_before: usize,
    diagnostics_after: usize,
    reused_percent: f64,
    computed_percent: f64,
    affected_percent: f64,
    time_ms: f64,
    invalidated_nodes: usize,
    reused_nodes: usize,
    recomputed_nodes: usize,
    total_nodes: usize,
    source_len: usize,
    tokens_total: usize,
}

fn status_payload(
    pipe: &LanguagePipeline,
    engine: &Engine,
    snap: &delta_text::PipelineSnapshot,
) -> StatusPayload {
    let r = engine.last_report();
    let reused =
        snap.tokens_reused + snap.ast_reused + snap.symbols_reused + snap.diagnostics_reused;
    let recomputed = snap.tokens_recomputed
        + snap.ast_recomputed
        + snap.symbols_recomputed
        + snap.diagnostics_recomputed;
    let total = (reused + recomputed).max(1);
    StatusPayload {
        inserted_chars: snap.inserted_chars,
        deleted_chars: snap.deleted_chars,
        tokens_reused: snap.tokens_reused,
        tokens_recomputed: snap.tokens_recomputed,
        ast_reused: snap.ast_reused,
        ast_recomputed: snap.ast_recomputed,
        symbols_reused: snap.symbols_reused,
        symbols_recomputed: snap.symbols_recomputed,
        diagnostics_reused: snap.diagnostics_reused,
        diagnostics_recomputed: snap.diagnostics_recomputed,
        diagnostics_before: snap.diagnostics_before,
        diagnostics_after: snap.diagnostics_after,
        reused_percent: (reused as f64 / total as f64) * 100.0,
        computed_percent: (recomputed as f64 / total as f64) * 100.0,
        affected_percent: r.affected_percent,
        time_ms: DurationMs(snap.elapsed_nanos).ms(),
        invalidated_nodes: r.invalidated_nodes,
        reused_nodes: r.reused_nodes.max(snap.ast_reused + snap.tokens_reused),
        recomputed_nodes: r.recomputed_nodes.max(recomputed),
        total_nodes: r.total_nodes.max(5),
        source_len: pipe.source.len(),
        tokens_total: pipe.tokens.tokens.len(),
    }
}

struct DurationMs(u128);
impl DurationMs {
    fn ms(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }
}

fn graph_ascii(engine: &Engine) -> String {
    let mut lines = vec![
        "SOURCE".into(),
        "  |".into(),
        "  v".into(),
        "TOKENS".into(),
        "  |".into(),
        "  v".into(),
        " AST".into(),
        "/ | \\".into(),
        "v  v  v".into(),
        "SYMBOL DIAGNOSTIC REFERENCES".into(),
        "".into(),
    ];
    for id in engine.graph().live_ids() {
        if let Ok(n) = engine.graph().node(id) {
            lines.push(format!(
                "  {} {} {}",
                n.name,
                n.version,
                status_word(n.status)
            ));
        }
    }
    lines.join("\n")
}

fn status_word(s: delta_core::NodeStatus) -> &'static str {
    match s {
        delta_core::NodeStatus::Clean => "clean",
        delta_core::NodeStatus::Dirty => "invalidated",
        delta_core::NodeStatus::Pending => "pending",
        delta_core::NodeStatus::Computing => "recomputed",
        delta_core::NodeStatus::Failed => "failed",
    }
}
