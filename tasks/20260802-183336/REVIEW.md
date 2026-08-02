# Review: Scaffold the standalone nova_autopilot crate

- TASK: 20260802-183336
- BRANCH: feat/nova-autopilot-crate

Worktree: `/home/alex/.cache/sprouts/nova-protocol/feat/nova-autopilot-crate`
Reviewed at `efd1e0ed`.

## Round 1

- REVIEWER: in-context (implementing session) - exception, see below
- VERDICT: APPROVE

**Exception.** The
standing session directive forbids dispatching subagents unless the user asks,
so the out-of-context round-1 default could not be met. Mitigation: every DoD
proof was rerun from scratch, and the load-bearing claim - "exactly one
dependency, `bevy`" - was re-derived independently of the grep proof, by reading
`cargo metadata` rather than the manifest text (`deps: ['bevy']`, `publish: []`,
`version: 0.9.1` inherited). A fresh `/flow 20260802-183336` session starting at
REVIEWING would satisfy the default if the owner wants a second pass.

### Findings

#### MINOR - `AGENTS.md:32` - crate code map is now incomplete [FIXED]

The repo `AGENTS.md` opens with "Read this file" and a code-map table with one
row per workspace crate, down to `nova_meta_gen`. This diff adds a fourteenth
crate and leaves the table at thirteen. The map is the declared entry point for
every agent touching this repo, so a crate whose entire purpose is to be the
destination for eight sibling port tasks should be listed in it.

Actionable change: add a row after `nova_probe`, e.g.
`| `nova_autopilot` | Automation drivers and the run-completion protocol; `bevy`-only. |`

Not raised higher: each sibling port task names the crate path explicitly in its
own record, so discovery is not actually blocked - only the shared map is stale.

### Verified

- Steps 1-4 each satisfied on re-read of the diff. Manifest matches the mandated
  shape (workspace-inherited `version`/`edition`/`license`, `publish = false`,
  one-line `description`, `[lints] workspace = true`, sole `bevy = { version =
  "0.19.0" }` with no features), and matches the house style of
  `crates/nova_events/Cargo.toml` the plan pointed at.
- Workspace member inserted at the alphabetical slot between `crates/nova_assets`
  and `crates/nova_core`. No `default-members` key added; the comment explaining
  its deliberate absence is untouched.
- `lib.rs` carries `#![warn(missing_docs)]` and states the boundary in both
  required directions - what the crate owns (drivers + completion protocol,
  `bevy` only) and what stays in `nova_debug` behind caller hooks (scenario
  presets, camera posing, rigid-body freezing, overlay hiding) - and names the
  `S: States + FreelyMutableState` generic as the reason no
  `nova_gameplay::GameStates` appears.
- All four module files plus `pub mod prelude {}` exist, each `pub mod`-declared
  with a doc comment. No items, no `use`, no re-exports - confirmed by reading
  every file in full, not by line count.
- `Cargo.lock` was committed with the new member, so a `--locked` CI build will
  not fail on a stale lockfile. Correct call; it was not in the Steps.

### Proofs rerun

| Proof | Result |
| --- | --- |
| `cargo check -p nova_autopilot` | pass |
| guarded anchored dependency grep | pass (exit 0) |
| `RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps` | pass, no rustdoc warnings |
| `cargo metadata --format-version 1 --no-deps` | pass (exit 0) |
| `cargo fmt --check` | clean |

The `nova_os` HUD `ambiguous_import_visibilities` warnings seen during the build
are pre-existing on `master` and outside this diff; not a finding here.

Full `cargo test`/`clippy` not run locally per the standing project policy; CI
covers them. Recorded as a skip, not as a pass.

### Verdict rationale

APPROVE. No BLOCKER or MAJOR. The single MINOR does not block; it is worth folding in
before the first port task reads the map.

Pending manual checks: none.

### Round 1 follow-through

The MINOR was folded in on the branch rather than deferred: `AGENTS.md` now
carries the `nova_autopilot` row after `nova_probe`. No other finding was open,
so the verdict stands unchanged.
