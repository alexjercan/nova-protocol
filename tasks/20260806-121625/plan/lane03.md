# L3 - Untrusted input, data loss and persistence

**Baseline: NEUTRAL.** Behavior-only.

Findings: **F06, F07** (data loss), **F08, F09, F12, F13** (input caps),
**F10** (`fire_rate` panic), **F14** (silent dispatch break), **F22** (settings
lost on quit), **F56, F57, F59, F60, F68, F69**, and **F61** once the owner has
ruled.

**Depends on:** L1, so a green probe run means something.

**Framing.** Mod content is untrusted input: it arrives from a remote portal
catalog and from files the player may have edited. A reachable panic, OOM or
stack overflow is a **defect**, not an upheld invariant.

## F06 + F07 - one failure mode, two halves. Land together.

F07 produces the corrupt file; F06 turns it into permanent loss on the next
install. Fixing either alone leaves a player who can still lose mods.

```rust
// crates/nova_assets/src/mod_cache.rs:512  (today)
pub fn read_index_at(root: &Path) -> Option<Vec<InstalledModRecord>> {
    let bytes = std::fs::read(root.join("installed.mods.ron")).ok()?;
    ron::de::from_bytes::<Vec<InstalledModRecord>>(&bytes).ok()
}
//   `None` conflates "no index yet" with "the index is corrupt", and
//   install_local_at:593 folds both into Vec::new() with unwrap_or_default().
```

```rust
// CHANGE  mod_cache.rs:512 - the two cases must be distinguishable
pub enum IndexRead {
    /// No index file. A first install writes a fresh one.
    Absent,
    Loaded(Vec<InstalledModRecord>),
    /// Present and unreadable. NEVER overwrite: doing so erases every other
    /// installed mod from DownloadedMods and orphans their bytes on disk
    /// where remove_mod can never sweep them.
    Corrupt(String),
}
pub fn read_index_at(root: &Path) -> IndexRead

// CHANGE  mod_cache.rs:582 install_local_at - refuse rather than clobber
pub fn install_local_at(
    root: &Path, id: &str, version: &str, bundle: &str, files: &[(String, Vec<u8>)],
) -> std::io::Result<()>
//   on IndexRead::Corrupt: side-band the bad file to installed.mods.ron.bad
//   and return Err. The install fails loudly; nothing is lost.
```

```rust
// NEW  crates/nova_assets/src/persist.rs - the atomic write, one helper
/// Write via temp file + fsync + rename, so a kill mid-serialize leaves the
/// previous file intact rather than a zero-length or half-RON one.
/// nova_probe/src/recorder.rs:213 and contract.rs:164 already carry the
/// correct pattern - this is that pattern, extracted.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()>
```

Four call sites, all bare `std::fs::write` today:

| Site | Loses |
| --- | --- |
| `mod_cache.rs:521` `write_index_at` | the installed-mod index (with F06: everything) |
| `persist.rs:91` `save_to` | `settings.ron` |
| `portal/catalog.rs:197` | the last-good catalog |
| `bin/content.rs:103` | generated content |

**Cluster (already identified).** These are **the same four files** as L10's
`Storage`-trait extraction. Sequence F07 as the change that *introduces the
trait's write contract*, not as a free helper L10 then has to absorb - see
`lane10.md`. `persist.rs` already splits `mod backend` by
`#[cfg(target_arch = "wasm32")]`, which is where the trait wants to live.

**Test:** kill mid-write, then install. One test covers both findings.

## F22 - the third persistence defect

```rust
// crates/nova_menu/src/settings.rs:247
//   The save is debounced 15 idle frames with NO flush on shutdown, and
//   save_settings has exactly one caller. Drag the volume slider, click Exit
//   (menu_ui.rs:564 writes AppExit immediately) within ~250 ms: setting lost.

// NEW  a flush system in `Last`, ordered before the AppExit drain
fn flush_pending_settings_on_exit(...)
```

**Cluster:** `settings.rs` also holds F26 and F27 (both L11). Whoever opens the
file should carry all three, even if the commits land in different lanes.

## The input caps - F08, F09, F12, F13

One slice of work, one owner, one test strategy: a hostile-RON corpus.

```rust
// CHANGE  crates/nova_mod_format/src/deps.rs:25  (F08)
fn visit(graph: &DepGraph, id: &str, seen: &mut HashSet<String>, out: &mut Vec<String>)
//   recurses once per graph edge with no depth bound, over a graph built from
//   untrusted catalog.json (install.rs:425). A stack overflow ABORTS the
//   process and cannot be caught, and it runs before validate_entry's caps.
+ const MAX_DEP_DEPTH: usize = 64;
+ fn visit(.., depth: usize, ..) -> Result<(), DepError>
//   PortalCatalog also needs an entry-count cap: MAX_FILE_COUNT bounds files
//   PER ENTRY, not entries.

// CHANGE  crates/nova_scenario/src/variables.rs:66 and filters.rs:164  (F09)
//   Both DSLs are Box-recursive with no depth limit in the RON decode or in
//   evaluate(). Deeply nested *.content.ron overflows the stack inside
//   ron::de::from_bytes ON THE ASSET-LOADER TASK DURING BOOT. The mod never
//   has to be enabled - the catalog loads every installed bundle's content.
+ const MAX_EXPR_DEPTH: usize = 32;   // enforced in decode AND in evaluate

// CHANGE  crates/nova_scenario/src/actions/spawn.rs:317 (field at :244)  (F12)
//   ScatterObjectsConfig::count is an unvalidated u32 driving an uncapped
//   spawn loop; lint/scenario.rs never inspects it. count: 50000000 OOMs from
//   data that passed both the static lint and the runtime gate. With
//   min_separation the rejection sampler is additionally O(count^2).
+ const MAX_SCATTER_COUNT: u32 = 4096;   // + a lint rule, so it fails early
+ // and an iteration cap on the rejection sampler

// CHANGE  crates/nova_assets/src/portal/catalog.rs:71 + transport.rs:31  (F13)
//   The body is read fully into memory with no size bound and parsed TWICE
//   (SchemaProbe, then PortalCatalog). The 256 KiB cap in last_good_store
//   gates persistence only, never the fetch.
+ pub const MAX_CATALOG_BYTES: usize = 1 << 20;
+ // bound the read at the transport, before either parse
```

## F10 - the asymmetry is already in the file

```rust
// crates/nova_gameplay/src/sections/turret_section/setup.rs:64  (spawn path)
let interval = 1.0 / muzzle.fire_rate;
let mut timer = Timer::from_seconds(interval, TimerMode::Once);
//   fire_rate is a plain required f32 on the serde-deserialized turret config.
//   0.0 gives +inf and Duration::from_secs_f32(inf) PANICS the moment the ship
//   spawns. Negative panics the same way.

// crates/nova_gameplay/src/sections/turret_section/setup.rs:192  (retune path)
let interval = 1.0 / muzzle.fire_rate.max(f32::EPSILON);
//   ^ the guarded form, one function away. Verified.
```

Fix: apply the same `.max(f32::EPSILON)` at `:64`, **and** lint `fire_rate` in
`lint/ship.rs` (which lints the hinge axis and muzzle presence but not this).
Belongs in this lane rather than with the section code: the root cause is an
unvalidated authored `f32`, same as F12.

## F14 - the scenario silently never advances

```rust
// crates/nova_events/src/engine.rs:170  (today)
pub fn from_data<T: serde::Serialize>(data: T) -> Self {
    let json_value = serde_json::to_value(data).ok();   // <- no log at any level
    Self { data: json_value }
}
//   EntityFilterConfig::filter (nova_scenario/src/filters.rs:71) reads
//   data: None as "does not match", so every entity-filtered handler for that
//   kind stops firing PERMANENTLY. Today's vocabulary is all-String, so this
//   is one added float field away from live.
```

```rust
// CHANGE  engine.rs:170
+ Err(e) => { error!("GameEventInfo::from_data: {e}"); }
//   and consider making the From<T> impl the only lossy path, with an
//   explicit try_from_data for callers that can handle failure.
```

## The rest

| Finding | Site | Change |
| --- | --- | --- |
| F56 | `nova_assets/src/merge.rs:214` | undeclared-ref violations are pushed **only** for `Content::Scenario`. A `Section`/`Campaign` with a bad `self://`/`dep://` ref is logged and merged anyway. Push for every content kind; the doc at `:145-148` already claims this |
| F57 | `nova_scenario/src/objects/binding_input.rs:83` | `HashMap` straight into serde output - **this writes `input_mapping:` into generated `assets/base/**/*.content.ron`**. `BTreeMap` or a sorted-key `serialize_map`. Same class at `lint_walk.rs:380,532` |
| F59 | `nova_assets/src/portal/mod.rs:176` | `install.entry.files[index]` guarded by `install.files.len() != index`, which does not bound `index` against `entry.files.len()`. Use `get(index)` |
| F60 | `nova_mod_format/src/deps.rs:104` | `cycle = order.len() != ids.len()` with un-deduplicated `ids`. Two records with the same id report "a dependency cycle" for a set with zero dependencies. Dedup `ids`, and reject duplicate ids in `mod_set.rs:222` |
| F68 | `nova_assets/src/mod_refs.rs:75` | `self://` refs rewrite via a raw string join, unlike membership-gated `dep://`. Defense-in-depth only - containment rests on `UnapprovedPathMode::Forbid` + `SandboxedAssetReader`. Gate it the same way |
| F69 | `nova_assets/src/portal/install.rs:459` | dependency installs fired-and-forgotten; the failed job is keyed under the **dependency's** id, not the dependent's, so the UI shows no linked surface. Key it under both |

**F57 gets its own commit.** Regenerating `assets/base/**/*.content.ron` is a
`content -- gen` run, never a hand-edit, and the generated churn would hide a
real diff in review.

**F61 - RULED 2026-08-07: epsilon compare.** `variables.rs:270`'s `Equal` node
is exact float equality; a mod author writing `Equal(hull_fraction, 0.5)` sees
the condition essentially never fire, with no error and no warning. The owner
ruled for an **epsilon compare inside `Equal`** - not a second `ApproxEqual`
node, and not documenting the sharp edge. Pick the epsilon by what the DSL's
values actually are (fractions, seconds, counts) and name it as a constant
beside the node rather than inlining a literal. Also a `modder`-persona
benchmark question candidate.

## Verified by

A hostile-RON corpus - malformed bundles, oversized catalogs, deeply nested DSL
expressions, duplicate ids, degenerate `fire_rate`. **This lane has the best
test story in the epic**: every finding is "authored data reaches code that
assumed it was sensible", so one fixture set covers most of it. Plus the
kill-mid-write test for F06/F07.
