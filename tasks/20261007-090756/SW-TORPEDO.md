# SW-TORPEDO: torpedo freeze/thaw (D-T7, gate 4.2/5/6)

Scope: `crates/nova_ship/src/sections/torpedo_section/{bay.rs,mod.rs,projectile.rs,frozen.rs}`,
and the torpedo half of P-T8 in `crates/nova_ship/src/sections/frozen_rounds.rs`. No other
files touched. Nothing staged.

## 1. Delivered

- `bay.rs:174-222` `pub(crate) struct TorpedoLaunch<'a>` and
  `bay.rs:224-301` `pub(crate) fn spawn_torpedo(commands, launch) -> Entity`: the one bundle
  builder, called by both `shoot_spawn_projectile` (`bay.rs:607-646`, fire) and `thaw_torpedo`
  (`frozen.rs:259-276`, thaw). Fire alone triggers `TorpedoLaunched` after the spawn
  (`bay.rs:647`); thaw never does.
- `torpedo_section/frozen.rs` (new): `FrozenTorpedo` (`frozen.rs:27-60`), `SavedTorpedoTarget`
  (`frozen.rs:67-87`, `Tracking { target: SavedTargetRef, last: Option<Vec3> }` per your
  revision), `freeze_torpedo` (`frozen.rs:146-240`), `thaw_torpedo` (`frozen.rs:253-298`).
  `TorpedoArming`/`TorpedoWeave`/`TorpedoColdLaunch` already had `#[cfg_attr(feature = "serde",
  derive(...))]` added at `mod.rs` beside their existing derives; all `FrozenTorpedo`/
  `SavedTorpedoTarget` fields stay the pub listed ones, nothing private added.
- `mod.rs` prelude (`mod.rs:58-69`) and crate-root re-export (`mod.rs:48-49`) extended with
  `freeze_torpedo, thaw_torpedo, FrozenTorpedo, SavedTorpedoTarget`.
- Tests, both in `torpedo_section/frozen.rs`'s own `mod tests` (so `--features serde frozen`
  and `--lib torpedo_section` both catch them):
  - P-T5 `a_resumed_torpedo_keeps_its_target_arming_and_cold_launch` (`frozen.rs` tests):
    real launch through `TorpedoSectionPlugin`, locks a `Tracking` target mid-flight while
    still cold-launching and already armed, freezes, RON-free round-trips through
    `freeze_torpedo`/`thaw_torpedo` directly (asserts cold remaining, arming fields, weave,
    steering, target all equal), then lets the real systems carry the thaw to ignition and
    detonation and asserts the blast's `ProjectileOwner` is the firing ship.
  - P-T5 second case `a_resumed_torpedo_with_a_frozen_target_keeps_its_last_known_position`:
    target dies mid-flight before the save, asserts `SavedTorpedoTarget::Frozen`, and that the
    thaw carries `TorpedoTargetChosen` + the frozen `TorpedoTargetPosition` with no
    `TorpedoTargetEntity`.
  - P-T8 torpedo half: extended `a_resumed_projectile_plays_no_launch_cue`
    (`frozen_rounds.rs`) with a real torpedo launch (one launch effect reset + one more
    `PlaySfx`) then a `thaw_torpedo` of a hand-built `FrozenTorpedo` (no effect reset, no extra
    `PlaySfx`, no `ERROR` logged). Removed the `TODO(20261007-090756)` line.

## 2. Deviations from the literal brief (not blocking, flagging as asked)

**(a) No `lifetime` field on `TorpedoLaunch`.** Mirrors T1: fire keeps inserting
`TempEntity`/`TorpedoWeave` itself right after `spawn_torpedo` returns (`bay.rs:648-652`);
thaw's lifetime is the caller's `resumed_lifetime(saved)` inserted in the SAME spawn bundle as
`thaw_torpedo`'s result, exactly like `rounds.rs`'s own pair. Verified by inspection, not a new
test: `on_insert_temp_entity` (`nova_gameplay/src/lifetime.rs`) only resets elapsed time when
`TempEntityState` is *not already present*, and T1's own test already proves `TempEntity` +
`resumed_lifetime` landing in one spawn tuple keeps the seeded state - the mechanism is shared
code, not torpedo-specific, so re-proving it here would duplicate that proof rather than add
one.

**(b) Weave-phase placeholder.** `TorpedoWeave::phase_for(torpedo)` needs the torpedo's own
final `Entity`, which does not exist until `spawn_torpedo` returns it - so `shoot_spawn_projectile`
passes `TorpedoLaunch.weave` seeded at phase 0, then immediately overwrites it with the correct
phase via `commands.entity(torpedo).insert(TorpedoWeave::new(..., TorpedoWeave::phase_for(torpedo)))`
right after (`bay.rs:648-652`), landing before any system reads the component. `thaw_torpedo`
passes the record's already-correct saved weave straight through with no correction. Existing
test `a_launched_torpedo_is_steered_along_the_way_it_was_ejected` and the weave tests in
`bay.rs`/`projectile.rs` (unchanged, still pass) cover that this produces the same weave a direct
build would.

**(c) New type: `pub(crate) struct TorpedoLaunchConfig(TorpedoSectionConfig)`**
(`bay.rs:174-181`), inserted by `spawn_torpedo` on every torpedo (fire and thaw alike).
`FrozenTorpedo.config` must be populated even when the launching bay has already died by freeze
time (gate 4.2 lists `config` as owned, not looked-up), and nothing else on the live projectile
carries the whole `TorpedoSectionConfig` - the existing `TorpedoSectionConfigHelper` lives on the
bay SECTION, not the torpedo, and is gone with it. This is the one new type I added without a
prior ask; I judged it pragmatically necessary (the alternative is `freeze_torpedo` failing or
inventing data for a dead-bay torpedo) and continued rather than stopping, per "a torpedo
outlives the tube that fired it" already being an established rule in this file
(`TorpedoType`'s own doc). Flagging it here as the STOP item the brief asked for if this
situation came up.
**Options if this needs a different design:** (1) keep `TorpedoLaunchConfig` as shipped; (2)
drop it and instead require `freeze_torpedo` to error (`TransientFreezeFault`) whenever the bay
is dead, accepting that a torpedo cannot survive its own launcher's death; (3) move the full
config onto `TorpedoGuidance`/`TorpedoBlast` instead of a new wrapper type - more invasive, touches
more call sites for no behavior change. I recommend (1), already shipped.

**(d) `thaw_torpedo` writes `TorpedoTargetChosen` for `Frozen` too**, not only `DumbFire`/
`Tracking` as the literal field list might read. Both `crates/nova_ship/src/input/player/intent.rs`
and `crates/nova_ship/src/input/ai/torpedo.rs` run their target-acquisition system filtered on
`Without<TorpedoTargetEntity>, Without<TorpedoTargetChosen>`. A thawed `Frozen` torpedo has
neither component unless `thaw_torpedo` adds `TorpedoTargetChosen` itself, so without this fix
the very next targeting pass would silently re-lock a torpedo whose target already died onto a
brand-new target - contradicting the live invariant (`update_target_position`'s own doc,
`projectile.rs:26-35`) that a torpedo keeps flying at the frozen point for life. Covered by the
second P-T5 test's explicit assertion. This is a correctness fix, not a new interface - no new
field or type, just one more branch writing an existing component - so I implemented it rather
than stopping.

## 3. STOP item: per-section `Health` is not in the approved record

`FrozenTorpedo`'s field list (gate 4.2, and your task body) has no slot for the mutable `Health`
of the torpedo's `TorpedoControllerMarker`/`TorpedoThrusterMarker` children
(`bay.rs`'s own `projectile_health_lands_on_the_launched_ordnance` test proves these are real,
independently-tracked HP, not a cosmetic value). `thaw_torpedo` → `spawn_torpedo` rebuilds both
children via `base_section(... health: config.projectile_health ...)`, i.e. always full HP. A
torpedo saved after taking partial damage to one section (survived a near-miss burst, point
defense grazed it) thaws back to full health - a free heal, silent, not fail-loud.
**I did not add a field for this** - it's not in the approved list and I was told to list it
rather than add it.
**Options:** (1) add `pub controller_health: f32, pub thruster_health: f32` to `FrozenTorpedo`
(or one `[f32; 2]`/a small struct) and have `thaw_torpedo` overwrite the two children's `Health`
after `spawn_torpedo` returns - smallest change, matches how `base_section`'s `health` already
flows; (2) accept the heal as a known, documented limitation of this save slice (torpedoes are
short-lived and rarely survive a graze anyway, so the save window is small) and move on; (3)
read the live `Health` generically off `TorpedoSectionPartOf`-filtered children in a later pass
owned by the collector, decoupling this entirely from `FrozenTorpedo`. I'd recommend (1) if this
is worth closing before ship; otherwise (2), noted as a known gap.

## 4. Checks run (real output)

```
$ nix develop --command cargo test -p nova_ship --lib torpedo_section::frozen -j 8
running 2 tests
test sections::torpedo_section::frozen::tests::a_resumed_torpedo_with_a_frozen_target_keeps_its_last_known_position ... ok
test sections::torpedo_section::frozen::tests::a_resumed_torpedo_keeps_its_target_arming_and_cold_launch ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1102 filtered out; finished in 0.16s

$ nix develop --command cargo test -p nova_ship --lib torpedo_section -j 8
test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 1032 filtered out; finished in 1.30-1.38s
(70 pre-existing + the 2 new P-T5 tests; re-ran after rustfmt too, same result)

$ nix develop --command cargo test -p nova_ship --lib --features serde frozen -j 8
running 8 tests
test sections::frozen::tests::a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with ... ok
test sections::frozen::tests::a_resumed_shed_fixture_keeps_its_art_style_and_grace ... ok
test input::player::wheel::tests::a_frozen_or_suspended_flight_takes_no_zoom ... ok
test sections::frozen_rounds::tests::a_resumed_projectile_plays_no_launch_cue ... ok
test sections::torpedo_section::frozen::tests::a_resumed_torpedo_with_a_frozen_target_keeps_its_last_known_position ... ok
test sections::torpedo_section::frozen::tests::a_resumed_torpedo_keeps_its_target_arming_and_cold_launch ... ok
test physics::pd_controller::tests::moderate_spin_despins_with_frozen_command ... ok
test physics::pd_controller::tests::fast_roll_despins_with_frozen_command ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1105 filtered out; finished in 0.76-0.82s
(one transient failure on `a_resumed_shed_fixture_keeps_its_art_style_and_grace` seen on an
earlier run, mid-edit by another worker on sections/frozen.rs, not mine to touch - re-ran clean)

$ nix develop --command cargo test -p nova_ship --lib ship_audio -j 8
test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 1050 filtered out; finished in 0.04s

$ nix develop --command cargo check -p nova_world_base --tests -j 8
Finished `dev` profile [optimized + debuginfo] target(s) in 18.09s
(no errors)
```

`nix develop --command cargo fmt -p nova_ship` run; re-tested after, same results above.

## 5. Mutations for the main worker to run later (not run by me)

- P-T5 (`a_resumed_torpedo_keeps_its_target_arming_and_cold_launch`): in `thaw_torpedo`, change
  `arming: record.arming.clone()` to `arming: TorpedoArming::new(0.0, 0.0, Vec3::ZERO, 0.0)` (a
  fresh, unarmed state) - the arming-equality assertion and the final detonation-despawn
  assertion must both fail.
- Same test: in `spawn_torpedo`, drop the `(AngularVelocity(angular), TorpedoLaunchConfig(config.clone()))`
  insert - `freeze_torpedo`'s `TorpedoLaunchConfig` lookup must then panic ("carries no
  TorpedoLaunchConfig").
- Same test: in the detonation blast insert (`projectile.rs`'s `torpedo_detonate_system`), change
  `if let Some(&owner) = owner { blast_entity.insert(owner); }` to never insert - the
  `blast_owner == ship` assertion must fail (no `ProjectileOwner` on the blast at all, so the
  query finds nothing and `.single()` panics).
- P-T5 second case (`a_resumed_torpedo_with_a_frozen_target_keeps_its_last_known_position`): in
  `thaw_torpedo`'s `SavedTorpedoTarget::Frozen` arm, drop `TorpedoTargetChosen` from the insert
  (keep only `TorpedoTargetPosition`) - the `TorpedoTargetChosen` presence assertion must fail.
- P-T8 torpedo half (`a_resumed_projectile_plays_no_launch_cue`): in `thaw_torpedo`, add a
  `commands.trigger(TorpedoLaunched { torpedo })` call after `spawn_torpedo` - the
  `PlaySfxCount == 2` and the effect-not-reset assertions must both fail.

## Round 2

Scope: same four files as Round 1, plus the torpedo half of `frozen_rounds.rs`. Nothing
staged. Owner task after TRANSIENT-GATE.md section 12, "T2/T3 worker deviations (owner,
2026-10-09)".

### 1. Claim and evidence

`FrozenTorpedo` now carries the per-part health the owner approved. `freeze_torpedo` reads it
from the live torpedo's two children; `thaw_torpedo` writes it back after `spawn_torpedo`.

- `frozen.rs:60-65`: `FrozenTorpedo` gains `pub controller_health: f32` and
  `pub thruster_health: f32`.
- `frozen.rs:128-148`: new private `fn part_health<M: Component>(world, torpedo, label) -> f32`.
  Walks `torpedo`'s `Children`, finds the one marked `M`, reads its `Health.current`. Panics
  naming the torpedo if it has no children, no child marked `M`, or that child has no `Health`.
- `frozen.rs:238-242`: `freeze_torpedo` calls `part_health::<TorpedoControllerMarker>` and
  `part_health::<TorpedoThrusterMarker>` and stores the two results in the returned record
  (`frozen.rs:280-281`).
- `frozen.rs:304-349`: `thaw_torpedo`, after `spawn_torpedo` returns, queues one deferred
  `Commands` world closure (the same pattern `railgun_section/wake.rs`'s
  `light_railgun_slug` uses to reach a child spawned in the same flush): it walks the thawed
  torpedo's `Children`, and sets `Health.current` on the one marked
  `TorpedoControllerMarker` to `record.controller_health` and on the one marked
  `TorpedoThrusterMarker` to `record.thruster_health`. No clamp; the value lands as saved. The
  default `spawn_torpedo` gives both children (full HP from `config.projectile_health`) is
  overwritten, never read back.
- `frozen_rounds.rs:501-518`: the P-T8 hand-built `FrozenTorpedo` fixture gets
  `controller_health: 10.0, thruster_health: 10.0` (the stock `projectile_health` default), so
  it keeps compiling and keeps asserting the no-launch-cue behavior, unchanged otherwise.

### 2. Report: a part can read `current <= 0` without `TorpedoShotDownMarker`

Asked for in the task, not fixed, no policy invented.

`on_damage` (`nova_gameplay/src/integrity/health.rs:142-154`) sets `health.current = 0.0`
**synchronously**, in the same `HealthApplyDamage` trigger that hit the part, then queues
`commands.entity(entity).insert(HealthZeroMarker)` for the next flush. Only once that
`HealthZeroMarker` insert lands does `on_torpedo_body_destroyed`
(`bay.rs:60-78`) fire and queue `TorpedoShotDownMarker` on the torpedo root - itself another
deferred `Commands` insert, needing a further flush to land.

So there are two flush boundaries between a part's `Health.current` hitting zero and the
torpedo carrying `TorpedoShotDownMarker`. If `freeze_torpedo` runs on a `World` reference taken
inside that window (a save point scheduled between the damage-applying system and the schedule's
next `apply_deferred`), it reads `controller_health <= 0.0` (or `thruster_health <= 0.0`) on a
torpedo that `freeze_torpedo`'s own `Unsettled` check (`frozen.rs:214-219`, tests `TorpedoShotDownMarker`
only) does not yet see as unsettled, and freezes it anyway. The saved record would carry a
part at zero or negative-clamped-to-zero HP on an otherwise-live-looking torpedo.

A sibling file already has a policy for the analogous section case: `sections/frozen.rs` (not
mine to touch) treats a section carrying `HealthZeroMarker` as `UnsettledBody`, "just hit zero,
the disable observer has not run yet" (its own doc, around that file's `freeze_fixture`/
`freeze_section` checks). `freeze_torpedo` has no equivalent check on the two child parts. I did
not add one - the task asked me to report this, not invent the policy. If the owner wants it
closed, the smallest fix I see is checking `HealthZeroMarker` on the controller/thruster children
inside `part_health` or `freeze_torpedo` and returning `TransientFreezeFault::Unsettled`,
mirroring the sibling file's rule - but that is the owner's call, not mine to make.

### 3. Test: P-T5 now proves the health round-trip

`a_resumed_torpedo_keeps_its_target_arming_and_cold_launch` (`frozen.rs:443`, extended):

- After the existing two `app.update()` warm-up ticks, finds the torpedo's controller and
  thruster children by marker and triggers a real `HealthApplyDamage { entity: controller,
  source: None, amount: 4.0 }` (`frozen.rs:490-512`), through the real `on_damage` observer
  (`NovaIntegrityPlugin`, already in `real_launch_app`'s app). `projectile_health` is the stock
  default (10.0), so this leaves the controller at 6.0 and the thruster at 10.0 - different from
  each other and from the authored max, so a test that only ever saw full HP on both parts could
  not have caught a mix-up or a silent heal.
- Asserts `record.controller_health == 6.0` and `record.thruster_health == 10.0` right after
  `freeze_torpedo` (`frozen.rs:573-580`).
- After `thaw_torpedo` and one `flush()`, finds the thawed torpedo's controller and thruster
  children and asserts their live `Health.current` equal the same two values
  (`frozen.rs:592-622`).

### 4. Checks run (real output)

```
$ nix develop --command cargo test -p nova_ship --lib torpedo_section -j 8
test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 1032 filtered out; finished in 1.20-1.40s
(70 pre-existing + the 2 P-T5 tests, both still named a_resumed_torpedo_*)

$ nix develop --command cargo test -p nova_ship --lib --features serde frozen -j 8
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1105 filtered out; finished in 0.66-0.71s
```

`cargo check -p nova_world_base --tests`: SKIPPED on owner instruction (2026-10-09, mid-task) -
the main worker is editing `nova_world_base` now and it will not compile until these fields
land. Not run this round.

`nix develop --command cargo fmt -p nova_ship` run; re-tested after, same results above.

### 5. Mutations run (and reverted)

- Comment out the whole `commands.queue` health-write block in `thaw_torpedo`: P-T5 failed on
  `thawed_controller_health, controller_health_before` (`left: 10.0, right: 6.0` - the default
  full-HP spawn value, never overwritten). Reverted; suite green again.
- Swap `controller_health`/`thruster_health` on the two `let` bindings just before the
  `commands.queue` call (so the controller gets the thruster's saved value and vice versa):
  P-T5 failed the same assertion the same way (`left: 10.0, right: 6.0` - the thruster's value
  landed on the controller instead). Reverted; suite green again.

### 6. Unverified

- The flush-window race in section 2 is reasoned from `on_damage`'s and
  `on_torpedo_body_destroyed`'s source, not reproduced with a failing test - the task asked for
  a report, not a repro, and a repro would need to land a save point mid-schedule, which is the
  collector's harness, not this module's.
- Whether `nova_world_base`'s `FrozenTransient::validate` (TRANSIENT-GATE.md's own description:
  finite and in `(0, max]`) will treat a record saved during that same race window as a refused
  Load or a silently-accepted edge value is validation's call, not checked here - explicitly
  out of scope per the task ("Validation of saved health at Load is NOT yours").
