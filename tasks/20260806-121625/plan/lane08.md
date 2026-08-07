# L8 - nova_probe restructure

**Baseline: BLOCKS - lands AFTER it.** Splits a crate and renames every module.

No findings - L1 already fixed the defects. This lane is the structure.

**Depends on: L1 - hard.** Restructuring a gate that is currently blind means
the restructure's own verification is unreliable. Also L2.

**The verdict from `../notes/04-nova-probe.md` stands on structure:** "rename,
do not rebuild". It was amended only on *confidence* - the loader in front of
that pipeline is what L1 fixes. **Do not let the rename absorb the fixes, and
do not let the fixes drift into a rename.**

## The real boundary

`nova_probe` is two programs separated by a **process boundary that no module
name states**: an in-game collection library linked into every example, and a
host CLI that spawns that example as a child process. The filesystem and env
vars are the IPC.

```
NEW crate  nova_probe       - in-game collection. Links into examples. wasm-clean.
NEW crate  nova_probe_cli   - host harness. Spawns children, reads artifacts,
                              renders reports. Never linked into a game binary.
```

## Module map

| Today | Goes to | New name |
| --- | --- | --- |
| `capture.rs` | nova_probe | `capabilities/frametime.rs` |
| `recorder.rs` | nova_probe | `capabilities/timeline.rs` |
| `invariants.rs` | nova_probe | `capabilities/invariants.rs` |
| `profile.rs` | nova_probe | `capabilities/profile.rs` |
| `contract.rs` | **both** | `contract.rs` - the wire format. See below |
| `stats.rs` | **both** | `contract.rs`'s neighbour: the CSV format both sides speak |
| `run_report/` | nova_probe_cli | `evaluation/` (artifacts, checks) + `report/` (html, manifest) |
| `aggregate.rs` | nova_probe_cli | `report/aggregate.rs` |
| `catalog.rs` | nova_probe_cli | `evaluation/catalog.rs` |
| `report.rs` | nova_probe_cli | `report/mod.rs` |
| `bin/probe/` | nova_probe_cli | `main.rs` + `native/` |
| `profile_sandbox.rs` | **evict** | host-side setup, belongs beside `supervise` |
| `fixtures.rs`, `run_report/fixtures.rs` | **evict** | test support, `#[cfg(test)]` or a dev-dependency |
| `bin/perf_web.rs` | **evict** | a separate tool, not part of either program |

`contract.rs` and `stats.rs` are the only genuinely shared code: the game
**writes** them, the host **reads** them. Either a third tiny crate
(`nova_probe_wire`) or `nova_probe` with `nova_probe_cli` depending on it -
prefer the latter until a second consumer exists (KISS).

**This split also deletes 20-odd `#[cfg(not(target_arch = "wasm32"))]`
attributes** (`lib.rs:82-163`). They exist only because host code and game code
share one crate; once they do not, the cfgs are the crate boundary.

```rust
// NEW  crates/nova_probe/src/lib.rs - the collection side, one plugin
/// Everything an example wires to be probeable. Per-example configuration
/// stays per-example; this bundles the capabilities rather than replacing
/// their individual registration.
pub struct NovaProbePlugin {
    pub frametime: bool,
    pub timeline: bool,
    pub invariants: bool,
}
```

## The prelude work - 13 of rule 3's missing 80

`nova_probe` has **12 public modules and zero preludes** - the only crate in
the workspace with public modules and no prelude at all, which is why its
deep-import count (184) is the workspace record.

```
NEW  one prelude per module in BOTH crates, written at the point the module is
     created under its new name - never added to the old names and then moved.
```

## The rename with a consumer outside the source

**`cargo run -p nova_probe -- run --all` becomes `-p nova_probe_cli`.** Every
caller moves in the same commit or CI breaks on a crate that no longer has a
binary:

```
CHANGE  .github/workflows/ci.yaml
CHANGE  AGENTS.md
CHANGE  any justfile / scripts/
CHANGE  every doc line quoting the command
```

**Grep for `-p nova_probe` before declaring the lane done.** This is the only
rename in the epic with a non-Rust consumer.

Also moving: `Cargo.toml` workspace members, the root dev-dependency, and the
`[[bin]]` entries (`nova_probe/Cargo.toml:18-30`).

## Sequence

1. Carve `nova_probe_cli` out with the module names **unchanged**, so the
   commit is a pure move and reviewable as one.
2. Rename to `capabilities/` `evaluation/` `report/` in a second commit.
3. Preludes and deep-import routing in a third.
4. Evictions last - they are the only ones that can be argued about.

## Verified by

`probe run --all` before and after, byte-comparing verdicts, **plus the
fixture-driven gate tests L1 introduced** - which is precisely why L1 must
precede this lane. A restructure verified only by the thing it restructured
proves nothing.
