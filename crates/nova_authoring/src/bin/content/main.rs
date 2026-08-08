//! The content authoring/validation CLI: one tool over the repo's content
//! tree, with a subcommand per task. Replaces the former separate
//! `gen_content`, `content_lint` and `balance_audit` bins.
//!
//! ```text
//! cargo run -p nova_authoring --bin content -- gen
//! cargo run -p nova_authoring --bin content -- lint [--target <mod-dir-or-id>] \
//!     [--report <path>] [--format md|html]
//! ```
//!
//! - `gen` writes the builder-backed base content files: the
//!   scenario/section builders in `nova_authoring::scenario_generation` are the
//!   single definition of each built-in; this serializes them into the committed
//!   `assets/base/**/*.content.ron` the game loads. Run it (and commit the
//!   result) after any builder change - the `content_ron_parity` test
//!   asserts the files match and names this command when they drift.
//! - `lint` runs EVERY content check in one pass: the identifier + geometry +
//!   resource checks the load/publish gates cannot make (unknown section
//!   prototypes, dangling NextScenario targets, unspawnable filter targets,
//!   duplicate ids, mount-base adjacency, resource-ref membership, canonical
//!   schemes), the combat balance/fairness audit (spawned-dead ERROR,
//!   close-spawn WARN, graded against `balance_acks.ron`; a stale ack is an
//!   ERROR), and the flight-rig input-overlap check (a content
//!   `input_mapping` section reusing a key the always-on flight rig binds
//!   silently double-drives flight). `--target`
//!   lints one mod: a mod directory anywhere on disk
//!   (the dir name is the mod id, portal-style) or an in-repo id
//!   (`webmods/<id>`, `assets/mods/<id>`, or `base`). `--report <path>`
//!   writes a per-mod document that pinpoints, for every finding, the file +
//!   element + explanation + suggested fix (`--format md|html`, Markdown the
//!   default; a `.html` path implies HTML). Exits non-zero on any Error
//!   (broken reference, spawned-dead, stale ack); CI runs the same walks via
//!   the `content_lint_gate` and `balance_audit_gate` tests. The `audit`
//!   subcommand was folded in here - balance is a kind of lint - so old
//!   `content audit` invocations become `content lint`.

// A NATIVE tool over the repo's content tree: `gen` writes files through
// `persist::write_atomic`, which is native-only by design. The wasm target
// gets a stub main so `cargo check --workspace --target wasm32` stays green
// over the lib graph the bundle actually ships.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    native::main()
}
