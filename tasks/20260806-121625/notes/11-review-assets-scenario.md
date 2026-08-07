# Code review - nova_assets, nova_scenario, nova_modding, nova_mod_format

Source: dedicated reviewer, 2026-08-07. Spot-verified against the tree.

**Framing that matters:** mod content is **untrusted input**. It arrives from
a remote portal catalog and from files on disk the player may have edited. A
panic, OOM or stack overflow reachable from mod data is a defect, not an
upheld invariant. Most findings below are that shape.

## Data-loss cluster - the most serious finding in the whole review

Three sites, one root cause, and they compound.

### 1. A corrupt index silently deletes every other mod

`crates/nova_assets/src/mod_cache.rs:593` (also `:104` `upsert_index_record`,
`:117` `remove_index_record`):

```rust
let mut records = read_index_at(root).unwrap_or_default();
```

A failed or corrupt read folds into an empty `Vec`. The next write persists
that empty base plus one record.

Failure: `<data_root>/installed.mods.ron` is truncated or malformed. Player
installs mod `B`. `read_index_at` returns `None`, `unwrap_or_default()` gives
`[]`, `write_index_at` writes `[B]`. **Mods `A`, `C`, `D` vanish from
`DownloadedMods` on the next boot.** Their bytes stay on disk as unreachable
orphans, and `remove_mod` can never sweep them because no record names them.

VERIFIED by read. Severity: bug.

### 2. Every persisted store is written non-atomically

`crates/nova_assets/src/mod_cache.rs:521`, and the same shape at
`persist.rs:91`, `portal/catalog.rs:197`, `bin/content.rs:103`:

```rust
std::fs::write(root.join("installed.mods.ron"), ron)
```

`std::fs::write` truncates before the new bytes land. No temp-file + rename,
no fsync.

Failure: the process is killed while serializing the index over the old file.
On restart the file is zero-length or half-RON, `read_index_at` returns
`None` - **and then finding 1 makes the loss permanent on the next install.**

The module doc claims a files-first-index-last discipline that "must leave a
readable state". A truncated index is not one. Same exposure for
`enabled_mods.ron` (whole mod selection reset) and `settings.ron`.

VERIFIED by read. Severity: bug.

The fix is one shared helper - write to `<path>.tmp`, fsync, rename - applied
at four call sites. Note `nova_probe/src/recorder.rs:213` already carries a
comment about exactly this hazard, so the codebase knows the pattern.

### 3. Duplicate ids report a phantom dependency cycle

`crates/nova_mod_format/src/deps.rs:104` computes
`cycle = order.len() != ids.len()`, but `ids` is not deduplicated.

Failure: the index carries two records with `id: "pack"` (reachable via a
hand-edited or partially-recovered index -
`mod_set.rs:227 start_downloaded_loads` validates each record but never
rejects duplicate ids). `ids = ["base","pack","pack"]`; Kahn emits `base`,
`pack`, skips the duplicate as already-emitted, so `2 != 3` and `cycle =
true`. The player sees "a dependency cycle among enabled mods prevents a full
topological order" **for a set with zero declared dependencies**, and the
recovery loop at `deps.rs:106` appends nothing because the id is in `emitted`.

`merge.rs:129` explicitly assumes ids are unique. Natural fix point is a
dedup in `start_downloaded_loads`.

Severity: bug. Confidence: likely.

## Unbounded-input cluster - reachable from a remote catalog

| # | Site | Failure |
| --- | --- | --- |
| 4 | `nova_scenario/src/actions/spawn.rs:317` (field at `:244`) | `ScatterObjectsConfig::count` is an unvalidated `u32` from mod RON driving an uncapped spawn loop, and `lint/scenario.rs` never inspects it. `count: 50000000` clones the template 50M times, queues 50M boxed closures, spawns 50M entities. Hangs then OOMs, **from data that passed both the static lint and the runtime content gate**. With `min_separation` set the rejection sampler is additionally O(count^2) |
| 5 | `nova_assets/src/portal/catalog.rs:71` + `transport.rs:31` | The catalog body is read fully into memory with no size bound before `serde_json` parses it - **twice** (once for `SchemaProbe`, once for `PortalCatalog`). The 256 KiB cap in `last_good_store` gates persistence only, never the fetch. A 2 GiB response OOMs the client before the schema gate can reject anything. `install.rs:181` acknowledges the per-file variant for downloads; the catalog request has no declared size to check against at all |
| 6 | `nova_mod_format/src/deps.rs:25` | `transitive_deps`'s `visit` recurses once per graph edge with no depth bound, over a graph built from untrusted `catalog.json` (`install.rs:425`). `PortalCatalog` has no entry-count cap (`MAX_FILE_COUNT` bounds files *per entry*). A 200k-entry chain overflows the stack on Install - **not a catchable panic**, and before `validate_entry`'s caps run |
| 7 | `nova_scenario/src/variables.rs:66`, `filters.rs:164` | Both DSLs are `Box`-recursive with no depth limit in the RON decode or in `evaluate`. A `*.content.ron` nested ~100k deep overflows the stack inside `ron::de::from_bytes` on the asset-loader task during boot. **The mod never has to be enabled** - the catalog loads every installed bundle's content as a dependency |

Findings 6 and 7 are the sharpest: stack overflow aborts the process and
cannot be caught, and 7 fires on an installed-but-disabled mod.

## Gate coverage gaps

### 8. Undeclared-ref violations are recorded for scenarios only

`crates/nova_assets/src/merge.rs:214`:

```rust
for message in mod_refs::resource_ref_violations(item, &scope) {
    error!("register_bundles: content {message}");
    if let Content::Scenario(cfg) = item {
        undeclared_ref_issues.push((cfg.id.clone(), message));
    }
}
```

A `Section` or `Campaign` with a bad `self://` / `dep://` ref is logged and
then **merged anyway**, so the runtime gate never sees it.

Failure: a mod ships a turret section whose `fire_sound` is
`dep://art/sounds/gun.wav` while `art` is neither declared nor enabled. The
section lands in `GameSections`, is offered in the editor palette, and
`rewrite_leaf` leaves the ref literal (`mod_refs.rs:87` returns `None`), so
the asset server fails on an unknown `dep` source at spawn time.

The doc at `merge.rs:145-148` claims this is "recorded as an Error content
issue ... so the runtime gate refuses it". True for scenarios only.

VERIFIED by read. Severity: bug.

### 9. `self://` refs get no component validation

`crates/nova_assets/src/mod_refs.rs:75` always rewrites via a raw string join,
unlike `dep://` which is membership-gated. `self://../../base/textures/x.png`
produces `mods/evil/../../base/textures/x.png`.

Containment currently rests **entirely** on bevy's `UnapprovedPathMode::Forbid`
default plus `SandboxedAssetReader` (`mod_cache.rs:342`). The ref layer
contributes nothing. Per finding 8, the `resource_ref_violations` scan that
would flag it is dropped for Section content.

Defense-in-depth gap, not a live escape. Severity: smell.

## Lower severity

| Site | Issue |
| --- | --- |
| `portal/install.rs:459` | Dependency installs are fired-and-forgotten with no join, so a dependent commits even when a transitive dep's download failed. Documented as accepted at `:452-458`, but the failed job is keyed under the dependency's id, not the dependent's, so the UI shows no linked surface |
| `nova_scenario/src/objects/area.rs:53` | `forget_area_occupancy` prunes only when the AREA despawns. A body destroyed *inside* a live area pins its count above zero forever (avian fires no `CollisionEnd` for a despawned collider - the module says so at `:49-51`). A scenario gating on `OnExit` never advances. `AreaOccupancy` is also never cleared by `teardown_scenario_entities` |
| `nova_scenario/src/lint/scenario.rs:291,348` | `(0.0..=MAX).contains(&secs)` admits `0.0` while the message claims `(0, MAX]`. `auto_advance_secs: Some(0.0)` lints clean, then `outcome.rs:217` builds a `Timer` that finishes on its first tick - the victory banner flashes past unread, and the lint that exists to catch this stays silent |

## Came back clean

- **The four `unreachable!()` in `lint/`** are inside `#[cfg(test)] mod tests`
  (opened at `ship.rs:314`, `scenario.rs:529`). Test assertion helpers. **This
  corrects `05-assets-scenario.md` and `08-tests-ci-risk.md`** - both are now
  amended. The only production `unreachable!()` is
  `nova_gameplay/src/mesh/slice.rs:67`.
- **Path traversal / zip-slip: unusually well done.** `is_safe_id` /
  `is_safe_rel_path` (`mod_cache.rs:134,142`) reject every non-`Normal`
  component, are applied in the shared `validate_file_op` **before** the cfg
  dispatch, and re-applied at the fs boundary in each `*_at`. `validate_entry`
  (`install.rs:231`) enforces a URL charset so `%2e%2e` cannot mean one thing
  locally and another on the wire.
- `Collider::sphere` with a RON-authored zero/negative/NaN radius is degenerate
  geometry, not a panic - verified against `avian3d-0.7.0` and `parry3d-0.27.0`
  (`Ball` has no assert).
- HTML report escaping (`content_report.rs:332`) escapes `&` before `<`/`>`,
  so mod-supplied names cannot inject markup.
- **Overlay precedence does not depend on `HashMap` iteration order** -
  `merge_bundles` consumes an explicitly ordered `Vec`, and `topological_order`
  re-scans `ids` in input order each round rather than draining a hash-keyed
  queue. The non-determinism hypothesis is dead.
- `portal/mod.rs:228` `.expect(...)` is guarded by the `get_mut` above.
- `percent_decode` (`config.rs:154`) bound is correct for a trailing `%XX`.

## Bearing on the epic

None of this is refactor fallout - it all exists today. But two items change
the epic's shape:

- The **atomic-write helper** (finding 2) touches the same four files the
  `Storage`-trait extraction in `05` would touch. Doing them together is
  strictly cheaper than doing them apart, and the trait is the natural home
  for "write atomically" as a contract rather than a convention.
- Findings 4, 6 and 7 are all **missing input caps on the modding surface**.
  That is a coherent slice of work with a single owner and a natural test
  strategy (a fuzz-shaped corpus of hostile RON), and it is orthogonal to
  every structural move. It wants its own task.
