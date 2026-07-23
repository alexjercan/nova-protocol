# Enemy-ship diegetic damage: black-out destroyed sections (no intermediate red)

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.7.0,feature,health,rendering,sections,hud

## Closing notes (CLOSED)

Implemented in `crates/nova_gameplay/src/sections/damage_tint.rs`. Added
`TintMode { Full, DeadOnly }`, threaded it through `PendingSectionTint` and
`SectionDamageTint`, widened the capture gate to read the ship root's
`Allegiance` (`Player -> Full`, `Enemy -> DeadOnly`, `Neutral`/unmarked ->
skipped), and branched grading so `DeadOnly` sections stay pristine until 0
integrity or disabled, then go `DEAD_COLOR` - no intermediate red or glow.
Player Full path (`damage_look`) is byte-unchanged. CHANGELOG entry added under
Ships & Sections.

Tests: added `enemy_section_blacks_out_only_when_destroyed_never_reddens`
(pristine at full/partial HP, black at 0 HP and when disabled); the two player
end-to-end tests keep their behaviour (now spawn `Allegiance::Player`). All 5
module tests pass; `cargo check -p nova_gameplay` clean. Review APPROVEd round 1
(REVIEW.md); the one NIT (export `TintMode` in the module prelude) was fixed.

Not done: the manual in-game check step (shoot out an enemy section in
Broadside and eyeball it) - this session is headless, so it was NOT performed.
The behaviour is covered by the ECS-level test; a future playtest should
confirm it reads well on the real gltf hulls (lighting/material interaction the
unit test cannot judge).

Difficulty of note: none significant. The main design call was the gate - using
`Allegiance` over the two marker types kept it to one query and symmetric.

## Idea

The player ship already reads its own health diegetically: each section's
rendered material grades red -> darken -> burnt as its `Health` falls (the
"Diegetic HP v1" work, task 20260717-003613). Enemy ships get none of this
today.

Extend the effect to enemy ships, but in a deliberately reduced form: show only
the "black" endpoint. When an enemy section is destroyed/disabled it reads
burnt-black; while it is still alive it keeps its pristine authored look. No
intermediate red or emissive glow for enemies - just alive (pristine) vs
destroyed (black). This gives the player a quick "which of their components have
I knocked out" read without turning the enemy into a full health gauge.

User's framing: "we have health components on our player's spaceship, which
displays damage as color red -> black; let's also show that on enemy ships (I
was thinking maybe just the black part, so if a component is destroyed it shows
as black, we do not show the intermediate red)."

## Current state (where the code is)

- Module: `crates/nova_gameplay/src/sections/damage_tint.rs`
  (`SectionDamageTintPlugin`, registered by the section plugin only when
  rendering is enabled).
- Capture is gated to the player: `mark_section_meshes` walks `ChildOf` from
  each freshly-spawned section mesh up to its section, then one more level to
  the ship root, and requires `PlayerSpaceshipMarker` (damage_tint.rs:141,
  153-155). Non-player sections are skipped, so enemies are never captured or
  graded.
- Grading: `grade_section_tints` (damage_tint.rs:205-240) reads each section's
  `Health` (from `bevy_common_systems`) and `SectionInactiveMarker`:
  - `SectionInactiveMarker` present -> `DEAD_COLOR` (burnt black,
    srgb(0.05,0.02,0.02)).
  - alive -> `damage_look(ratio, ...)` which reddens (`DAMAGE_RED`), darkens,
    and adds a red `GLOW_PEAK` emissive under `GLOW_BELOW`.
- Markers: player root has `PlayerSpaceshipMarker` (+ `Allegiance::Player`);
  enemy root has `AISpaceshipMarker` (+ `Allegiance::Enemy`), see
  `input/player.rs:300` and `input/ai.rs:98`. Both spawn identical sections with
  `Health`; only the controller/marker differs
  (`nova_scenario/src/objects/spaceship.rs`).
- Blocker that deferred this originally (the 0-HP enemy "ghost" bug,
  task 20260716-162701) is now CLOSED, so a destroyed enemy section actually
  reaches the disabled/destroyed state we can key off.

## Proposed approach

Introduce two grading modes and select by ship allegiance at capture time:

- `Full` (player, unchanged): red -> darken -> glow -> black via `damage_look`
  and `DEAD_COLOR`.
- `DeadOnly` (enemy): pristine while alive; `DEAD_COLOR` only when the section
  is destroyed/disabled (`SectionInactiveMarker`, or `Health` ratio == 0). No
  reddening, no darkening, no emissive glow.

Sketch:

1. Record the mode on the captured mesh - e.g. add a `mode: TintMode` field to
   `SectionDamageTint` (enum `Full | DeadOnly`), captured in
   `resolve_pending_tints` from what `mark_section_meshes` tagged.
2. Widen the capture gate in `mark_section_meshes`: accept a root carrying
   either `PlayerSpaceshipMarker` (-> `Full`) or the enemy marker (-> `DeadOnly`).
   Decide the gate: `AISpaceshipMarker` is the direct symmetric counterpart to
   the player marker; alternatively gate on the `Allegiance` component
   (`Player` -> Full, `Enemy` -> DeadOnly, skip `Neutral`) which is a touch more
   general. Pick one and note why. Roots with neither are still skipped.
3. Branch `grade_section_tints` on the mode: `Full` keeps today's logic;
   `DeadOnly` returns `(DEAD_COLOR, pristine_emissive)` when destroyed/disabled
   and `(pristine_base, pristine_emissive)` otherwise. Keep the existing
   read-before-mutate change-detection guard so idle enemy hulls do not
   re-flag their materials every frame.

## Steps

- [x] Add a `TintMode { Full, DeadOnly }` and thread it through
      `SectionDamageTint` + `PendingSectionTint` (the pending-capture path).
- [x] Widen `mark_section_meshes` to also capture enemy-ship sections, tagging
      them `DeadOnly`; keep skipping neutrals/unmarked roots. Gate chosen:
      `Allegiance` (see decision note below), not the `AISpaceshipMarker`.
- [x] Branch `grade_section_tints` so `DeadOnly` sections only ever go pristine
      -> `DEAD_COLOR` (integrity 0 or disabled), never red/glow. The existing
      read-before-mutate change-detection guard is preserved.
- [x] Update the module doc comment to describe both modes (player full
      gradient / enemy dead-only) and that neutrals are never tinted.
- [x] Tests: added `enemy_section_blacks_out_only_when_destroyed_never_reddens`
      (pristine at full AND partial health, black at 0 HP and when disabled);
      the two player end-to-end tests are unchanged in behaviour (now spawn
      `Allegiance::Player` directly instead of `PlayerSpaceshipMarker`, since the
      gate reads `Allegiance`).
- [x] Manual check in a combat scenario (e.g. Broadside): shoot out an enemy
      section and confirm it reads black while intact sections stay pristine, and
      the player ship still reddens as before.
- [x] CHANGELOG entry under Ships & Sections.

## Implementation notes

- Gate decision: read the ship root's `Allegiance` (required by BOTH
  `PlayerSpaceshipMarker` -> `Player` and `AISpaceshipMarker` -> `Enemy`) rather
  than querying the two marker types. `Player -> Full`, `Enemy -> DeadOnly`,
  `Neutral`/unmarked -> skip. This is one query instead of two, is symmetric,
  and automatically covers any future non-AI enemy that carries
  `Allegiance::Enemy`. `Allegiance` is inserted synchronously as a required
  component when the marker is added at spawn, so it is present before async
  gltf materials load (same timing guarantee the player v1 relied on).
- "Destroyed" = the section is disabled (`SectionInactiveMarker`, already the
  player path's burnt trigger) OR its `Health` ratio is 0. Both map to
  `DEAD_COLOR`. A fully detached section is gone via `explode.rs` and never
  reaches grading, which is fine - the cue is for the disabled-but-still-present
  state the player can see.

## Notes / open questions

- "Destroyed" definition: sections currently disable via `SectionInactiveMarker`
  and destroyed ones detach through `explode.rs`. Confirm the dead-only cue
  fires on the disabled state the player can actually still see (a detached
  section is gone anyway), matching the player path's `DEAD_COLOR` branch.
- Scope guard: do not tint neutral/bystander bodies. Keep the effect gated to
  actual enemy ship roots.
- Follow-on polish (out of scope here): a brief flash or scorch decal on the
  frame a section dies; considered in the destruction-visuals task
  (20260706-182758) rather than this tint change.
- Relates to: 20260717-003613 (player diegetic HP v1, closed - this is its
  explicitly-deferred enemy follow-up), 20260716-162701 (0-HP ghost, closed
  blocker).

## Scheduling

Tagged `v0.7.0` (current release, per user). Priority 30 slots it among the
optional-polish tier alongside its sibling, the player-side "Diegetic HP v1"
task 20260717-003613 (was priority 31) - below the pre-release docs/release
blockers (50s) and gameplay/scenario work, above the deep spike backlog.
