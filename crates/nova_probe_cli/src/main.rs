//! `probe` - the run-harness front door. One command runs autopilot examples
//! through the harness passes and hands back per-example run reports plus an
//! aggregated status index. A single example is the same aggregate shape with
//! one row:
//!
//! ```text
//! cargo run -p nova_probe_cli -- run player_path            # clean + declared frame time + trace + report
//! cargo run -p nova_probe_cli -- run player_path,scenario_grammar   # comma list -> aggregate
//! cargo run -p nova_probe_cli -- run ui                  # a category dir's examples
//! cargo run -p nova_probe_cli -- run --all               # the whole catalog
//! cargo run -p nova_probe_cli -- report <run-dir>           # re-render (manifest-gated)
//! ```
//!
//! `run` orchestrates natively: pass 1 CLEAN (timeline + invariants + log), a
//! frame-time pass when the program declares it, pass 2 PROFILED (a separate
//! trace build whose overhead never touches frame-time numbers), an optional
//! `--samply` flamegraph run, then
//! the run report in-process. Specs resolve against the Cargo.toml
//! `[[example]]` catalog (the single source of truth - autoexamples is
//! off), run sequentially with continue-on-failure, and write
//! `index.html` + `index.json` + `probe-all.json` above the per-example
//! run dirs.

fn main() -> std::process::ExitCode {
    nova_probe_cli::native::main()
}
