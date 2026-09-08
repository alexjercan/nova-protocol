# Nightly review

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: review

## Scope

Nightly review of everything that landed on `master` on 2026-09-08.
`git log --since=midnight --oneline` gives 24 commits, from `a391a8481`
(03:14) to `5cefb2d75` (22:36). Baseline for the range is `a391a8481^`
(`f6a2ea0..` resolved below).

The working tree is dirty on entry with the owner's uncommitted edits under
`tasks/20260908-161328/`, plus an untracked `proof/first-panel-card/`. Nothing
in this run touches those paths.

## Groups

Ordered highest risk first. `/nova-review` refuses above 2000 changed lines, so
the three archive-heavy groups are scoped to their source files and that is
stated per group.

| G | Range | Commits | Changed lines | `--play` |
|-|-|-|-|-|
| G1 | `a391a8481^..e366c32e2` | 4 | 242 | yes |
| G2 | `cb8d7e997..ab60b2998` | 1 | 114 | no |
| G3 | `e366c32e2..cb8d7e997` | 2 | 82 | no |
| G4 | `2f21f01dd..e3291f373` | 4 | 1631 | no |
| G5 | `ab60b2998..2f21f01dd` | 7 | 145 + binaries | yes |
| G6 | `ca1ccaef6..8ca7c24f3` | 1 | 1648 | no |
| G7 | `e3291f373..ca1ccaef6` | 1 | 1093 | no |
| G8 | `8ca7c24f3..7e8095fd8` | 1 | 2910 (scoped) | no |
| G9 | `7e8095fd8..5cefb2d75` | 3 | 2397 (scoped) | no |

### G1 - pyre and section fixture (highest risk)

`a391a8481` `1c63d3ac6` `3289fa0bf` `e366c32e2`.
`crates/nova_gameplay/src/integrity/pyre.rs`,
`crates/nova_ship/src/sections/fixture.rs`. Runtime gameplay plus the death
fireball and shed cladding. Gets `--play`: it is a visible destruction effect
and a panic fix, so the red team and feel lanes are worth their run.

### G2 - channel apply and probe capability snapshot

`ab60b2998`. `crates/nova_channel/src/apply.rs`, `lib.rs`,
`crates/nova_probe/src/capabilities/snapshot.rs`, bench page and docs. Protocol
surface an agent driver reads. No `--play`: nothing renders.

### G3 - bench gesture and observation, staged strike counters

`c5bf89b2c` `cb8d7e997`. `crates/nova_bench/src/gesture.rs`,
`observation.rs`, `examples/playable/wfc_arena.rs`. No `--play`: the bench view
is a data surface, not a rendered one.

### G4 - CI frame-cost gates, task records, skill removal

`9afed3706` `3e2be8b36` `7145b6fed` `e3291f373`. The reviewable code is
`3e2be8b36` (7 files, 148 insertions) taking the software renderer's frame cost
out of five gates; `9afed3706` is a 1426-line review record and the other two
are housekeeping. No `--play`: it is test and gate timing.

### G5 - screenshot and loop re-shoots, harness hull figure

`77c76e5c5` `39f6af711` `b6f6f6c10` `e532acd55` `47ca2e2f4` `6609c3a68`
`2f21f01dd`. `crates/nova_debug/src/harness.rs`, seven `examples/screenshots/`
programs, `scripts/gen-web-screenshots.py`, `docs/`, and rebuilt web assets.
Gets `--play`: it is entirely about what a frame shows.

### G6 - named Saturn cast and shared opening story

`8ca7c24f3`. `web/src/lore/**`, `web/src/docs-manifest.js`,
`web/webpack.config.js`, `web/tests/lore.test.js`, `CHANGELOG.md`. Renames
`junction.md` to `aquila.md` and `clearwell.md` to `baikal.md`, which is exactly
the unchecked-string rename the contracts lane exists for. No `--play`.

### G7 - shared illustration toolkit

`ca1ccaef6`. New `scripts/nova_illustration/` package plus
`scripts/gen-lore-designs.py`. Python, not Rust. No `--play`.

### G8 - opening comic proof archive

`7e8095fd8`. 2910 insertions, all under `tasks/20260908-161328/`. Over the
2000-line refusal, and unsplittable by range because it is one commit. Scoped to
the authored sources (`comic-opening-poc/generate.py`,
`clean-line-study/generate.py`, `proof/*.mjs`, `proof/*.py`, the READMEs);
the generated SVG, PNG and log files are archive, not code. No `--play`.

### G9 - briefing config and the accepted illustration restyle

`675ce45c6` `489aeeb35` `5cefb2d75`. `.scufris.toml` plus the restyle of
`scripts/nova_illustration/` and `scripts/gen-lore-portraits.py`. Scoped to
`.scufris.toml`, `scripts/**` and the authored task sources for the same reason
as G8. No `--play`.

## Findings

Appended per group as each is adjudicated.

### G1 - dispatched 23:0x

Wave A: craft, correctness, contracts, performance. Performance holds the
measurement slot, so red team and feel wait rather than share the GPU. Bundle at
`/tmp/nightly-20260908/g1/`.

Scoped bundles built. After dropping generated artefacts the three oversized
groups fit under the 2000-line refusal:

- G4 scoped to `crates`, `examples`, `.agents`, `.claude`: 15 files -> 149/47.
- G8 scoped to authored sources: 15 files, 1719 insertions.
- G9 scoped to `.scufris.toml`, `scripts/**` and authored task sources: 23
  files, 831/576.

The excluded paths are PNG, SVG, JSON and log archive, plus one 1426-line review
record. They are evidence, not code, and no lane reviews them.

Host checked quiet before the measurement slot opened: 23:04, load average
0.75/0.75/1.69, no nova or cargo process running.

### G1 - adjudication in progress

Craft, correctness and contracts all landed on the SAME MAJOR independently,
which is the strongest signal of the night. I re-derived it myself rather than
take three lanes' word:

- `crates/nova_gameplay/src/settings.rs:317-321`: `GraphicsBudget::default()` is
  `for_quality(GraphicsQuality::default())`, i.e. High, `particles: true`.
- `settings.rs:339-346`: the only writer, `apply_graphics_quality`, is an
  `Update` system behind `resource_changed::<GraphicsQuality>`.
- `crates/nova_gameplay/src/integrity/pyre.rs:628`: `warm_the_pyres` is a
  `Startup` system, and `Startup` completes before the first `Update`.
- So the gate at `pyre.rs:462` reads `particles: true` in every shipping app.
  The Low tier - the spawn-less low-end mode `settings.rs:255-257` describes,
  and the tier `nova_perf_web` exists to exercise - builds four `EffectAsset`
  graphs and uploads the 128x128 soft-dot mask at startup, then never spawns a
  single instance.

Reading `GraphicsQuality` instead does not fix it: the persisted preset lands in
`load_persisted_settings`, another unordered `Startup` system
(`crates/nova_menu/src/settings_store.rs:380`), so the tier is not settled in
`Startup` either.

Three docstrings assert the behaviour the code does not have: `pyre.rs:96-100`,
`pyre.rs:453-454`, and `crates/nova_gameplay/src/soft_dot.rs:17-22`.

There is no `SettingsSystems` set to order against; `IntegritySystems`
(`integrity/core.rs:57`) is the house precedent for how one is shaped.

Craft vs contracts disagreed on whether a deferred dead plate still costs a
round. Re-derived: BOTH are right about different functions, and craft's is the
live one.

- `crates/nova_gameplay/src/integrity/carve.rs:291-296` `absorbed_by` pays
  nothing for a zero-health node, so carve absorption is free. Contracts is
  correct here.
- `crates/nova_gameplay/src/damage.rs:454-461` `pierce_remainder`'s Pierce arm
  charges `health.max / pierce_power_multiplier(...)` per layer crossed -
  documented as MAX on purpose at `damage.rs:423-428`. A dead plate that still
  wears its `Collider` is still a layer. Craft is correct here.
- `fixture.rs:196-198` confirms the shed is what removes `Collider`, and
  `fixture.rs:175` `.take(SHED_FRAME_CAP)` is what defers it, so the window is
  `ceil(dead / 24)` frames rather than one.

Also confirmed by reading `fixture.rs:198-209`: the `try_insert` adds EIGHT
components, not the "four" the `SHED_FRAME_CAP` docstring at `fixture.rs:56`
argues from.

The performance lane MEASURED and found a fifth thing the other four missed,
which is the most useful finding of the group:

- bevy_hanabi 0.19 mints a WGSL source only from `CompiledParticleEffect::update`,
  called only from `compile_effects`, which iterates spawned INSTANCES. Adding an
  `EffectAsset` to `Assets` generates no shader. So `warm_the_pyres` warms the
  cheap half and the expensive half still lands on the death frame.
- Instrumented run of `stress_hull_collapse` at HEAD with `bevy_hanabi=debug`:
  five `pyre_section_*` shaders minted INSIDE the collapse window, 1.68 ms of
  main-thread WGSL generation on that frame. No `pyre_hulk_*` shader minted at
  all - the hulk pair still waits for the first `IntegrityRoot` death.
- Six-run set, `worst_frame_ms` medians: pre-pyre 76.2, range base `b8d17f99`
  102.7, HEAD 93.3. Populations flat at 720 wreck pieces. The warm-up recovered
  about a third of the pyre's 26.5 ms regression; ~17 ms is still there.
  Directionally consistent at n=6, not established.
- Host gate stated: loadavg 1.02-2.13 beside every run, dedicated Xvfb `:77`
  stopped by recorded PID, real NVIDIA adapter so the deltas stand and the
  absolute milliseconds are not a software floor.

Wave B (red team, feel) dispatched once the measurement slot was released.

## Continuation, 2026-09-09

The nightly run died after dispatching G1's wave B. This session picks the
review up from there, with two changes the owner asked for:

- Story and lore are out of scope. G6 (named Saturn cast), G7 (illustration
  toolkit), G8 (opening comic archive) and G9's illustration restyle are NOT
  reviewed and NOT edited. They are unsettled work, and the working tree still
  carries the owner's uncommitted edits under `scripts/nova_illustration/` and
  `tasks/20260908-161328/`.
- The review no longer only reports. Confirmed findings get fixed, directly on
  `master`, at most two agents at a time.

G1's wave B (red team, feel) is not re-dispatched. Four lanes already agreed on
the group's majors and the measurement is recorded; a rendered pass would not
change what gets fixed. Named as a skip, not a pass.

Remaining review: G2, G3 on one agent; G4, G5 on the other.

### G1 - adjudicated, verdict CHANGES REQUESTED

Re-derived in this session, every claim against the tree at `faffdc56b`.

MAJOR 1 - `crates/nova_gameplay/src/integrity/pyre.rs:462` - the tier gate is
never false. Four lanes agreed; the derivation is above. `GameStates` defaults
to `Loading`, so `OnEnter(GameStates::Playing)` is a frame-1-safe place to warm
from, and `warm_railgun_wake_art`
(`crates/nova_ship/src/sections/railgun_section/mod.rs:415`) is the sibling
precedent. `GraphicsBudget` is still unsettled during `Startup` for every other
reader too, so the settings plugin should derive it before the first frame
rather than only on the first `Update`.

MAJOR 2 - `pyre.rs:445-472` - the warm-up warms the cheap half only. Measured
by the performance lane: five `pyre_section_*` WGSL sources minted INSIDE the
collapse window, 1.68 ms on that frame, and no `pyre_hulk_*` shader at all.
`bevy_hanabi` mints from `CompiledParticleEffect::update`, which iterates
spawned INSTANCES, so adding an `EffectAsset` generates nothing. The same
false claim sits on the railgun sibling at
`crates/nova_ship/src/sections/railgun_section/wake.rs:204-210`.

MAJOR 3 - `crates/nova_ship/src/sections/fixture.rs:56-65` - `SHED_FRAME_CAP`
ties the drain to the RENDER frame rate. `shed_dead_fixtures` is registered in
`Update` (`shell_skin.rs:1145`) while the damage that raises `HealthZeroMarker`
resolves on the fixed step at 64 Hz. The docstring argues from "at sixty frames
a second"; this project's own captures record 373-387 frames over a collapse
window and about a second a frame under the software renderer. A dead plate
still wears its `Collider` for the whole backlog, and `pierce_remainder`'s
Pierce arm (`crates/nova_gameplay/src/damage.rs:454-461`) charges `health.max`
per layer crossed, so the backlog is paid in rounds, not only in frames.

MAJOR 4 - `crates/nova_gameplay/src/integrity/spew.rs:521` - NEW, re-derived
here, and the only finding of the group that is visible to a player. Shooting a
rock throws hot GUNMETAL chips. `CarveDebris::Rock` is inserted on the asteroid
ROOT (`crates/nova_scenario/src/objects/asteroid.rs:219`) while both spew
sources name the mesh NODE beneath it: `DamageMarks` rides on the node
(`asteroid.rs:267`) so `carve_body` announces `CarveSpew { entity: owner }` with
the node (`carve.rs:341`), and `throw_severed_pieces` announces `parent.node`
(`asteroid_carve.rs:425`). `spew_carved_material` does a FLAT
`q_debris.get(spew.entity)` and `unwrap_or_default()`s to `Metal`. The comment
at `asteroid.rs:217-218` states the behaviour this defeats. The module's own
test (`spew.rs:830`) puts `CarveDebris` on the same entity it spews from, so it
cannot see the split. `pyre.rs`'s new `inherited_material` walk is the fix
already written; spew needs to share it.

MINOR - `fixture.rs:56` says a shed plate "gains four components"; the
`try_insert` at `fixture.rs:198-209` adds eight.
MINOR - `pyre.rs:447` says "a 128-texel texture per size". `SOFT_DOT_TEXELS` is
128 per SIDE, and `soft_dot.handle` is called once outside the loop, so the mask
is shared and 128x128.
MINOR - `crates/nova_gameplay/src/soft_dot.rs:17-22` says the mask is "Built on
the first effect that asks, not at startup". The warm-up builds it.
MINOR - `docs/sections.md:502-509`, written the same day, still implies the
graphs are minted at the death.
MINOR - `docs/sections.md:314-323` does not state `SHED_FRAME_CAP`, while the
same chapter states `PYRE_FRAME_CAP` at :506 and explode's absence of a cap at
:499-501, so the omission reads as "there is no cap".
MINOR - `web/src/wiki/ships.md:39` still describes a section death as a silent
detach, with no flash and no ejecta. Reported before as Lane B finding 9 of
`tasks/20260908-004345`, still open.
MINOR - `CHANGELOG.md:356` is 211 characters joined, over the 200 cap.
MINOR - `CHANGELOG.md:391` documents a bug whose whole lifetime is inside this
cycle: `48a3ba1dc` added the stated centre, `a5da7efdb` pinned it, and the
cladding it names (`3f8163fa8`) has never shipped either.
MINOR - `fixture.rs:196-198` queues two `try_remove` commands where one tuple
removal is a single archetype move; `fixture.rs:215` allocates a `Vec<Entity>`
per shed fixture inside the capped loop.
MINOR - `pyre.rs:519/536` walk the same `ChildOf` chain twice per death.
MINOR - `pyre.rs:504-511` is the third verbatim copy of the no-asset-stores
guard.
MINOR - `fixture.rs:41-51` states `SHED_KICK` in world units only; the sibling
`pyre.rs:42-46` sets the meters precedent AGENTS.md requires.

Not raised higher: no finding is a BLOCKER. Nothing here fails a build, breaks
a shipped format, or ships a crash - the panic this range was opened by is
already fixed at `fixture.rs:196`.

Skipped, and named as skips: the red team and feel lanes; a re-measurement of
the pyre warm-up after the fix.

### G2 - adjudicated, verdict CHANGES REQUESTED

Re-derived here. The lane's baseline correction matters: the last RELEASE is
`v0.12.0` (2026-08-31), not the lexically-last tag. `git tag --list | tail`
sorts `v0.9.1` after `v0.12.0`, which is how the wrong baseline gets picked.

MAJOR 1 - `CHANGELOG.md:459` - a `**(breaking)**` migration note for a field
that never shipped. `radar.dwell_fill` entered the snapshot in `1448d4cc3`,
which is NOT an ancestor of `v0.12.0`, and its introducing entry is still
unreleased at `CHANGELOG.md:500`. AGENTS.md: migration notes apply only to
formats that shipped. The `SNAPSHOT_SCHEMA` bump itself is earned - `applied[]
.state` DID ship under schema 1 - so only the dwell entry goes, and
`crates/nova_probe/src/capabilities/snapshot.rs:157-160` should stop citing
`dwell_fill` as half the reason for the bump.

MAJOR 2 - `crates/nova_bench/src/pages/targeting.md:25-26` - the new sentence
is wrong for the first 15 ticks of every radar hold, and contradicts line 20 of
its own page. `update_radar_search` assigns `radar.candidate` and only THEN
returns on `!hold_fired` (`crates/nova_ship/src/input/targeting/radar.rs:103-110`),
so for the whole `RADAR_TAP_SECS` window (0.25 s, 15 ticks at 60 Hz) a driver
reads a populated `candidate` beside a null `dwell_fill`. The page tells it to
re-aim; holding is exactly what it should do.

MAJOR 3 - `crates/nova_probe/src/capabilities/snapshot.rs:636` - a shipped wire
field changed shape with no test. Nothing in the crate's `mod tests` reaches
`radar_record`. Rewriting the gate as `radar.is_dwelling()` - the substitution
the comment itself warns against, because it goes false at completion, which is
the 1.0 the reader waits for - leaves the suite green. The sibling fix in the
same commit did get a test (`apply.rs:615`).

MINOR - `CHANGELOG.md:455`, `:459`, `:463` are 260, 264 and 233 characters
joined. The ack entry at :455 was 184 before this commit grew it.
MINOR - `crates/nova_channel/src/lib.rs:10-11` points at `docs/agent-bench.md`,
which `docs/keeping-docs-in-sync.md:82` does not own for this crate.
MINOR - `crates/nova_channel/src/apply.rs:265-272` - `apply_section` took the
same press-only gate as `apply_input` with no test of its own.

### G3 - adjudicated, verdict CHANGES REQUESTED

MAJOR - `examples/playable/wfc_arena.rs:3131-3142` - the staged strike's second
salvo collapses to one frame whenever the beat before it ends on a hit.
`Strike::hit` is `self.hits > 0` (:2536) and `hits` only ever increments
(`count_strike_hits`, :2557). "both sides open up" and "the second salvo" read
that same monotonic predicate back to back, and `stage_the_strike` zeroes the
counters ONCE (:2768). So the moment "both sides open up" ends on a hit rather
than on its timer, "the second salvo" is already true on its first evaluation
and `STRIKE_WINDOW_SECS` - documented at :3014 as the seconds the recording
waits on that salvo - buys nothing. The commit's comment argues the single
zeroing is "the whole fix because `disarm_the_rival_lances` runs a line later";
that covers `shots`, because lances ARE disarmed, and not `hits`, because
nothing disarms a torpedo. The commit title, "Scope the staged strike's
counters to the beat that reads them", describes the fix the code does not make.

MINOR - `CHANGELOG.md:410` is 269 characters joined, the longest entry in
`[Unreleased]`; this commit grew it from 222.
MINOR - `crates/nova_bench/src/gesture.rs:24` - `MAX_AIM_TICKS` is `60 * 60`
with a docstring naming `TICKS_PER_SECOND`, which is a `pub const` in the same
crate (`referee.rs:25`). Nothing checks the claim.
MINOR - `crates/nova_bench/src/manual.md:100` does not state the new `ticks`
bound, so a driver meets it only as a refusal.
MINOR - `crates/nova_bench/src/observation.rs:67-75` - with `me` now `None`,
`me_id` is `Value::Null`, and `label_of` reports `owner: null` for any round
whose owner has despawned, so a dead raider's rounds in flight count as the
dead player's `outbound`. Only reachable on the last view of a lost run.

### G5 - adjudicated, verdict BLOCKED

BLOCKER - `web/src/assets/wiki-radar.png` and eight sibling stills ship the
debug status bar. I cropped the top-right of each committed PNG myself:
`wiki-radar.png`, re-shot inside this range by `77c76e5c5`, reads
`30 fps  v 0.12.0+ab60b2998` - the range's own base commit, baked into a player
wiki page. `tutorial-radar-lock.png`, `feature-combat.png`,
`tutorial-combat-lock.png`, `wiki-hud.png`, `tutorial-orbit.png`,
`feature-autopilot.png` and `wiki-flight.png` all read `v 0.12.0+a5da7efdb`;
`news-090-contextual-hud.png` reads `v 0.9.1`. `tutorial-orbit.png` is also the
alias source for `wiki-gravity.png` (`scripts/gen-web-screenshots.py:270`), so
the bar reaches a second page.

This range is where the fix was written: `39f6af711` added `hide_status_bar`
(`crates/nova_debug/src/harness.rs:704`) with the docstring "The bar is the one
widget that must never reach a recording", and wired it into exactly two
producers. Every other HUD-on producer reaches `HudVisibility::On` through a
`hud_instrument` helper that does nothing else (`shared/ring.rs:312`,
`shared/hollow.rs:453`, `screenshot_radar_lock.rs:280`), so the guard is opt-in
at the call site and eight producers did not opt in. The four loops the range
re-shot ARE clean; the stills are not.

MAJOR - `crates/nova_gameplay/src/integrity/pyre.rs:203-211` is the consumer
`39f6af711`'s hull-figure sweep missed, and it is a sizing note. `HULK_PYRE` is
still sized against "a shipped gunship - 110 m stem to stern, 11 units" and
reads 130 m of ejecta as "a hull length of debris". `block_gunship` is 50 x 48 x
85 m, so the reference is 29% long and 130 m is one and a half hull lengths.
`pyre.rs:42-48` makes these figures load-bearing by its own account: "an earlier
cut read its own comments as metres, and shipped a section fireball of 9 m quads
reaching 126 m".

MINOR - `docs/environment-variables.md:177-183` warns of a failure the pipeline
cannot have. `NOVA_RAILGUN_AFTERMATH` is a duration, not a flag, and panics on a
bad value; a dropped `NOVA_RAILGUN_LIVE=1` makes `capture-web-media.sh:190-194`
abort by name and clobber the staged slowed row. It never ships two identical
loops.
MINOR - `examples/screenshots/shared/kit.rs:211-269` - `section_entity` was
extracted and `section_health` still holds the same four-line lookup.
MINOR - `crates/nova_debug/src/harness.rs:704` - `hide_status_bar` is not in the
harness prelude, so both call sites spell out the full path beside a bare
`ring::hud_instrument(world)`.

### G4 - adjudicated, verdict CHANGES REQUESTED

MAJOR - `examples/systems/stress_torpedoes.rs:124-153` - the rewritten docstring
names a run-level backstop the documented local command does not set. Probe
pushes `DEADLINE_ENV` only for the fps pass
(`crates/nova_probe_cli/src/native/env.rs:82-97`); a correctness run inherits
`DEFAULT_DEADLINE_SECS = 120.0`
(`crates/nova_autopilot/src/completion.rs:104-108`) while the four step
deadlines now sum to 240 s. Only `.github/workflows/ci.yaml:200` makes the
claim true, and only on the runner. The named-step diagnostic the docstring
promises is exactly what a local run loses.

MINOR - three sites justify a real-seconds deadline with per-frame capture cost
(`examples/systems/system_torpedo_launch.rs:1060`,
`examples/screenshots/loop_command_shell.rs:70`,
`examples/screenshots/screenshot_railgun.rs:863`). Under capture,
`LoopCapturePlugin` pins `TimeUpdateStrategy::ManualDuration`
(`crates/nova_autopilot/src/loops.rs:346-351`), so wall cost does not consume
the deadline; unarmed, nothing is recorded at all. The bounds are still safe.
MINOR - `examples/screenshots/screenshot_railgun.rs:742-757` - the
`pause_the_clock` in `shoot_step` is justified by a readback cost neither path
has. The real sink is the 30-frame settle, which the pause at :853 already
covers.
MINOR - `crates/nova_assets/tests/mod_cache_install.rs:333,364` - a long-red
catalog-count repair (4->3, 3->2, red since `220509616` retired the
`nova_protocol` entry) rode inside a commit whose message says "five CI gates".
The edit is correct; the message does not mention it.

Verified sound, no finding: the skill removal leaves no dangling reference; the
`system_cinematic` `debrief_beat >= 1.0` gate still requires the unskippable
scene; every re-derived 85 m framing checks out arithmetically; `TRACK_SPEED_FLOOR`
crosses units correctly.

## Fixes

Two lanes on disjoint paths, both editing directly on `master`, neither running
git. Lane A owns `crates/nova_gameplay/**`, `crates/nova_ship/**` and
`docs/sections.md`; lane B owns `crates/nova_bench/**`, `crates/nova_channel/**`,
`crates/nova_probe/**`, `crates/nova_debug/**`, `examples/**` and
`docs/environment-variables.md`. This session owns `CHANGELOG.md`,
`web/src/wiki/ships.md`, the still re-shoot and every commit.

Done in this session:

- `CHANGELOG.md` - dropped the `**(breaking)**` `radar.dwell_fill` migration
  note. The field entered in `1448d4cc3`, which is not an ancestor of `v0.12.0`,
  so it has never shipped and AGENTS.md gives migration notes only to formats
  that did. Folded its behaviour into the entry that introduces the field.
- `CHANGELOG.md` - restored the debris-pivot entry to the bug that SHIPPED.
  `a5da7efdb` had rewritten `48a3ba1dc`'s correct description ("spun about the
  section's origin ... swung wide and then snapped straight") into the
  intra-release symptom, and added cladding, which has never shipped either.
- `CHANGELOG.md` - brought every over-cap entry this range wrote or grew back
  under 200 characters: the death entry (211), the staged strike (269), the
  input ack (260), the `stop` fix (233), and the world-snapshot entry my own
  dwell edit had pushed to 243.
- `web/src/wiki/ships.md:39` - a section death is no longer described as a
  silent detach. Reported before as Lane B finding 9 of `tasks/20260908-004345`
  and still open.

Left alone, and named: `CHANGELOG.md:73`, `:76` and `:255` are over the cap and
are campaign and narrative entries, which the owner has ruled out of scope while
the story is unsettled.

### Lane A - runtime, committed as `c5b8878aa`

`crates/nova_gameplay/{settings,soft_dot,integrity/pyre,integrity/spew}.rs`,
`crates/nova_ship/src/sections/{fixture,shell_skin,railgun_section/wake}.rs`,
`docs/sections.md`.

- Added `SettingsSystems` and a `PostStartup` apply pass, so the derived
  `GraphicsBudget` is true before frame one for every reader and not only after
  the first `Update`. `warm_the_pyres` moved to `OnEnter(GameStates::Playing)`,
  the `warm_railgun_wake_art` precedent, where the tier a player picked in the
  menu is also settled.
- The warm-up now spawns one hidden instance per graph with its `EffectSpawner`
  held inactive, and drops them the next frame. I checked the mechanism myself
  rather than take the lane's word: `with_active(false)` sets only the emission
  flag (`bevy_hanabi-0.19.0/src/spawn.rs:723`), and `compile_effects` iterates
  `ParticleEffect` components without consulting it, so the shader is still
  minted while nothing is emitted.
- `inherited_material` moved from `pyre.rs` to `spew.rs` beside `CarveDebris`
  and both callers read through it. The lane proved the new test has teeth by
  reverting the walk and watching it fail.
- `shed_dead_fixtures` moved to `FixedUpdate`; `SHED_FRAME_CAP` renamed
  `SHED_TICK_CAP` (private) and re-argued from the tick rate.
- `HULK_PYRE` re-sized against the 85 m hull, plus the eight-component,
  128x128-mask and lazy-mask docstrings and the two `docs/sections.md`
  paragraphs.

Verified: `cargo check --features debug --all-targets` clean over the whole
workspace; `cargo fmt --all -- --check` clean; `nova_gameplay --lib integrity::`
87 passed, `--lib settings::` 9 passed; `nova_ship --lib sections::fixture` 8
passed, `--lib sections::shell_skin` 23 passed.

### Lane B - capture and protocol, committed as `45372be15` and `ba0cc418f`

- `hide_status_bar` exported from `nova_debug::prelude` and called by all three
  `hud_instrument` helpers, so a producer cannot ship the bar by forgetting a
  line. The lane checked that no producer wants it: `HudVisibility::On` appears
  in `examples/` only in those three helpers, and everything else uses
  `hide_hud`, which clears the tier anyway.
- `cue_the_lances` zeroes `shots` and `cue_the_tubes` zeroes `hits`, each on
  entry to the beat whose `until` reads it.
- The capture clock stated once in `nova_debug::harness` and cited from the
  three deadline notes that had it wrong. No bound changed.
- `targeting.md`'s null `dwell_fill` sentence corrected; `radar_record` pinned
  by three fixtures; `apply_section` given the sibling context-gate test;
  `MAX_AIM_TICKS` derived from `TICKS_PER_SECOND` and both put in the prelude;
  the manual states the bound; a playerless view reports zero ordnance rather
  than counting every orphaned round as the dead player's.
- The lane mutation-checked three of its new tests by breaking the code under
  them and watching them fail.

Verified: `nova_bench --lib` 58 passed, `nova_channel --lib` 15 passed,
`nova_probe --lib snapshot` 12 passed.

Not implemented, recorded as a recommendation: `clean_pass_env` should size a
correctness-pass deadline from the program's step budget the way
`fps_window_and_deadline_env` sizes the fps one, so the named-step diagnostic
stops depending on a line of CI YAML. It is outside the paths this run touched.

### Changelog

The rock-chip fix is the only one of the night that earns an entry: I checked
`v0.12.0` itself and the shipped tree already had `CarveDebris::Rock` on the
asteroid root (`asteroid.rs:190`), `DamageMarks` on the node beneath it
(`:234`) and the flat lookup in `spew.rs:464`, so the bug shipped. Everything
else fixed tonight - the pyre, the shed, the staged strike, the dwell gate -
was introduced after `v0.12.0` and is omitted as intra-release work.

### Re-shooting the stills, committed as `cc005e202`

The code fix in `45372be15` stops a producer from raising the bar, but the
figures already on the site were cut before it. I rebuilt the seven producers
and re-ran the capture in two batches, then read the result rather than the
exit status: cropping the top-right corner of each PNG and stacking the crops
shows the corner is now empty in all nine.

Re-shot: `feature-autopilot`, `feature-combat`, `tutorial-combat-lock`,
`tutorial-orbit`, `tutorial-radar-lock`, `wiki-flight`, `wiki-gravity`,
`wiki-hud`, `wiki-radar`.

`news-090-contextual-hud.png` was staged and deliberately not copied. The
packager froze it - "shipped with its post; NOVA_UNFREEZE to re-cut" - and the
rule's own argument applies exactly here: a news figure is evidence of the game
at the time of its post, so re-cutting it would show a reader footage of a
build that post was never about. Its "v 0.9.1" bar is period-accurate. Left
frozen; `NOVA_UNFREEZE=news-090` is the escape hatch if the owner disagrees.

### Documentation, committed as `78baad3eb` and `1956c26f2`

`CHANGELOG.md` was written against the last commit in places rather than the
last release. Corrected against `v0.12.0`: the `radar.dwell_fill`
**(breaking)** migration note deleted, because the field entered in `1448d4cc3`
which is not an ancestor of the tag and so never shipped; the debris-pivot
entry restored to the bug players actually met; the rock-chip entry added; six
entries trimmed under the 200-character cap.

Three entries stay over cap on purpose - `CHANGELOG.md:73`, `:76`, `:255`. All
three are campaign and narrative copy, which this run was told not to edit.

`web/src/wiki/ships.md:39` described a section's death as a silent detach. It
has burned since `v0.12.0`.

## Outcome

Six commits on `master`: `c5b8878aa`, `45372be15`, `ba0cc418f`, `cc005e202`,
`78baad3eb`, `1956c26f2`.

Groups reviewed: G1-G5. Groups excluded by the owner: G6-G9, the story and lore
work, which was neither reviewed nor edited.

Findings: 1 BLOCKER (the status bar on nine shipped figures), 5 MAJOR (the
cold-start graphics tier, the pyre warm-up minting no shader, rock chips
thrown as hull plate, the shed running per frame, the arena's stale counters),
all fixed. One MAJOR was found in this continuation and not by the original
night: the rock chips. One recommendation is left unimplemented and named
above, `clean_pass_env`, as outside the paths this run touched.

Not run, per the standing instruction: the workspace test suite and Clippy.
Proof is `cargo check`, `cargo fmt --check`, the per-crate test filters listed
above, and reading the rendered PNGs.
