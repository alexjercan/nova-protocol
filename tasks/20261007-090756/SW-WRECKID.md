# SW-WRECKID: wreck-root `EntityId` mint on sever

Scope: `crates/nova_ship/src/sections/integrity.rs` only (and its inline
tests). Design: TRANSIENT-GATE.md "D-T7 amendment" + "D-T7 second amendment"
(B: live-only collision check).

## 1. Mint implementation

Claim: `sever_disconnected_structures` now mints a fragment's `EntityId` once,
at its birth, from the sever source's id and the fragment's least section id;
a missing source id, a sectionless fragment, or a live collision leaves the
fragment id-less and logs `error!`; an existing wreck's id is never touched.

Evidence / change:
- `crates/nova_ship/src/sections/integrity.rs:496-504` — new private
  `fn wreck_id(source: &EntityId, sections: &[&EntityId]) -> Option<EntityId>`.
  `None` iff `sections` is empty (`min_by_key` on the inner `String`, then
  `format!("{}/wreck/{}", source.0, least.0)`).
- `:515-545` — `q_roots` gained `Option<&EntityId>` (the source id), `q_sections`
  gained `Option<&EntityId>` (each section's id), and a new read-only
  `q_live_ids: Query<&EntityId>` param for the collision check.
- `:549-563` — destructure updated for the new `source_id` field; three other
  `q_sections.get(...)` destructures at the old :557/:595 lines gained one more
  `_` for the new tuple field (the two that used `..` needed no change).
- `:663-683` (post-fmt) — per fragment, in the existing spawn loop:
  ```rust
  let section_ids: Vec<&EntityId> = component
      .iter()
      .filter_map(|section| q_sections.get(*section).ok().and_then(|(.., id)| id))
      .collect();
  match source_id.and_then(|source| wreck_id(source, &section_ids)) {
      None => error!(/* names fragment + root; no source id or no section id */),
      Some(id) if q_live_ids.iter().any(|live| live.0 == id.0) => error!(/* names
          fragment, root, minted id, and the source id */),
      Some(id) => { commands.entity(fragment).insert(id); }
  }
  ```
  An existing wreck (the retained component, `index == retained`) is `continue`d
  past before this block runs, same as before — its id (if any) is never
  recomputed because this code only runs for newly-spawned fragments.

Blast radius: one system (`sever_disconnected_structures`) in one file. No
public signature changed; `wreck_id` is private. No other `nova_ship` file
touched.

Verification: `cargo check -p nova_ship` clean. See `tests` section below.

## 2. Consumer sweep (report only, no edits)

None of the following change gameplay or saving because a wreck root can now
carry an `EntityId`. Verdicts:

- **`UnownedBody` sector classification** —
  `crates/nova_world/src/frozen.rs:622,798` key the snapshot/thaw filters on
  `Without<ScenarioAddressableMarker>`, not on `EntityId` presence. Minting
  `EntityId` does not insert `ScenarioAddressableMarker`. **No change.**
- **Scenario lookups by id** — `nova_scenario/src/actions/mod.rs:87-105`
  (`scoped_entities`/`scoped_entity`) filter on
  `ScenarioAddressable = (With<ScenarioScopedMarker>, With<ScenarioAddressableMarker>)`.
  A wreck gets neither marker from this change. **No change.**
- **Radar / HUD / target labels** —
  `crates/nova_interface/src/map/contacts.rs:191-230` classifies contacts from
  `ships` (`With<SpaceshipRootMarker>`) and `terrain` (requires
  `&EntityTypeName`); a `ShipWreckFragmentMarker` root matches neither, so it
  is never a contact and `sort_key`'s `self.ids.get(entity)` (:263-270) is
  never reached for one. **No change.**
- **AI target filters** — `nova_ship/src/input/ai/{behavior,threat,frozen,mod}.rs`
  hold no `EntityId`-keyed query or filter (confirmed by grep; only
  `passive.rs` references `EntityId`, and only inside test fixtures). **No
  change.**
- **`nova_probe` snapshot** — `crates/nova_probe/src/capabilities/snapshot.rs:1005-1016`,
  `label`/`label_of`, and the body `"id"` field at `:1335`, read
  `world.get::<EntityId>(entity)` unconditionally for ANY body, wreck included.
  A wreck's `"id"` field and any reference to it (e.g. `"ai_target"`,
  `"combat_lock"`) will print the minted string instead of falling through to
  `Name`/`null`. This is diagnostic/proof output, not gameplay or saved state,
  so it is **not a stop condition** — but it **does change** recorded probe
  snapshots that happen to reference a wreck. Flagged for whoever owns probe
  reference snapshots next.
- **`live_by_id` in `nova_world_base/src/save/transients.rs:391`** — the one
  call site is `live_by_id::<With<SpaceshipRootMarker>>`, and
  `resolve_all`'s only owner variant is `SavedOwner::Ship(id)` (:384-391); a
  wreck fragment never carries `SpaceshipRootMarker`, so it is invisible to
  this filtered query regardless of id. **No change** (this is the seam T2/T3
  widen later, per the base D-T7 rule's own note).
- **Frozen-body freeze/thaw** — `crates/nova_world/src/frozen.rs:424-481`
  (`freeze_body`) captures `id: body.get::<EntityId>().cloned()` (:471)
  GENERICALLY for every `FrozenBodyType`, including `WreckFragment`
  (`body.contains::<ShipWreckFragmentMarker>()` branch at :463 falls through
  to the same generic `id` line). `thaw_record` (:494-539) re-inserts it
  generically at :521-523 for any body, wreck included. **`FrozenBody.id`
  already carries a minted wreck id, and thaw already reinserts it on the
  wreck root, with no change needed in `nova_world`.** This was the open
  question in the amendment text; it is answered by existing generic code,
  not by any edit in this change.

No consumer requires an edit outside `integrity.rs`, and none of the above
changes gameplay or saved-state behavior, so step 3 proceeded without a stop.

## 3. Tests

All three live in `crates/nova_ship/src/sections/integrity.rs`, `mod
physics_tests`, immediately after `destroying_an_interior_bridge_severs_a_physical_wreck`:

- `a_severed_ship_mints_a_distinct_id_for_each_wreck` — a ship `"ship_a"`
  (controller section `"left"`, plus `"mid"` and `"right"` each isolated by a
  destroyed bridge in the same frame) splits into 3 components in one sever;
  asserts the 2 wreck roots carry distinct `ship_a/wreck/...` ids and the ship
  keeps `"ship_a"`.
  - Mutation that should kill it: drop the `Some(id) => { commands.entity(fragment).insert(id); }`
    arm (never insert) — both `app.world().get::<EntityId>(fragment)` reads
    panic on `.unwrap()`. Or: make `wreck_id` always return the same literal —
    `assert_ne!(ids[0], ids[1])` fails.
- `a_wreck_severed_again_derives_its_id_from_the_wreck` — one bridge sever
  produces wreck `"ship_a/wreck/rear"` (least of `{right, second_bridge,
  rear}`); a second cut inside that wreck produces a grandchild whose id is
  asserted to equal `format!("{wreck_id}/wreck/{grandchild_section_id}")`.
  - Mutation that should kill it: in `wreck_id`, swap `source.0` for a
    constant (e.g. always use the ship's original id instead of whatever
    `source` the call site passes) — the grandchild assertion
    (`format!("{wreck_id}/wreck/...")`, where `wreck_id` is the WRECK's id, not
    `"ship_a"`) fails because the grandchild id would read `ship_a/wreck/...`
    instead of `ship_a/wreck/rear/wreck/...`.
- `a_minted_id_already_live_leaves_the_wreck_idless` — calls
  `sever_disconnected_structures` directly via `run_system_cached` off a
  hand-built `PendingSeverRoots` entry (not through `app.update()`/`trigger`),
  because a scheduled update runs the multi-threaded executor, which a
  thread-local `tracing_subscriber::fmt().set_default()` capture steps around
  — the same reason `a_sever_that_never_gets_mass_data_retries_once_then_expires_with_one_line`
  (same file, pre-existing) calls `apply_pending_sever_motion` directly rather
  than through `app.update()`. A decoy entity pre-holds
  `EntityId::new("ship_a/wreck/right")`; asserts the log contains `"ERROR"`,
  the new fragment has no `EntityId`, and the decoy keeps its id unchanged.
  - Mutation that should kill it: remove the `q_live_ids.iter().any(...)`
    guard (always take the `Some(id) => insert` arm) — the fragment gets
    `EntityId("ship_a/wreck/right")` (duplicate with the decoy), the log has no
    `"ERROR"`, and the "stays id-less" assertion fails.

Not run (per instructions): mutation testing itself. The main worker runs it.

## 4. Checks run (real output)

```
$ cargo test -p nova_ship --lib integrity
running 47 tests
...
test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 1055 filtered out; finished in 0.11s
```
(includes all 3 new tests: `a_severed_ship_mints_a_distinct_id_for_each_wreck`,
`a_wreck_severed_again_derives_its_id_from_the_wreck`,
`a_minted_id_already_live_leaves_the_wreck_idless`, all `ok`.)

```
$ cargo test -p nova_ship --lib --features serde frozen
running 5 tests
...
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1105 filtered out; finished in 0.66s
```

```
$ cargo check -p nova_world -p nova_world_base --tests
    Checking nova_world v0.15.0 (...)
    Checking nova_world_base v0.15.0 (...)
    Finished `dev` profile [optimized + debuginfo] target(s) in 28.63s
```
(no warnings or errors from either crate)

```
$ cargo fmt -p nova_ship
```
ran clean (reformatted `integrity.rs`); all four checks above were re-run
AFTER the format pass and are the output lines quoted here.

Note: one retry was needed before the `nova_world`/`nova_world_base` check and
once for the post-format `nova_ship` test run — `cargo check`/`test` first hit
`error[E0583]: file not found for module 'frozen'` in
`crates/nova_ship/src/sections/torpedo_section/mod.rs:47`, a file I do not
own. Per instructions I did not touch it; I waited and retried, and it
resolved on its own (the other worker's `frozen.rs` landed) — the file now
exists and every check above passed clean on retry.

## Unverified / not checked

- Did not run `cargo test -p nova_ship` (full crate) or any workspace-wide
  test/Clippy, per instructions.
- Did not verify behavior under `nova_probe`'s own test suite for the snapshot
  format-change noted in section 2 — flagged, not fixed, as it is outside
  `integrity.rs`.

## Main worker check

- Accepted. Removed the "D-T7 ..." task citations from the three test docs
  (AGENTS.md: no task citations in durable docs).
- The probe label change has no committed reference: `git ls-files` holds
  no probe snapshot reference, and no non-Rust tracked file has
  "Severed Ship Wreck".
- Mutations named here are queued for the serial mutation pass.
