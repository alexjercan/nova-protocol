# Decisions and deviations

Recorded while implementing TASK.md. Each entry states what was chosen and why
the task's stated rule did not settle it on its own.

## Kept base content the task did not list

- `block_wreck_plate` stays in base. `season_one/stage.rs:171` spawns it as the
  chapter's debris field, so it has a shipped consumer. The other three carrier
  wrecks (`block_wreck_bridge`, `block_wreck_shoulder`, `block_wreck_spine`)
  had none and are deleted.
- `block_picket` stays in base. It is the hull the console lookup tests and the
  NOVA OS terminal tests address by name, and the bench hostiles now fly it.

## Consumer substitutions

- Bench hostiles were `block_raider`; they now fly `block_picket`. The raider
  left the catalog and the picket is the nearest armed hull of the same class.
- The arsenal bench's ram target was `block_carrier`; it is now `block_hauler`,
  the largest hull still in base.

## Ordering

- The generated section catalog order changed. It follows the family split
  (`hull`, `thruster`, `controller`, `turret`, `torpedo_bay`, `railgun`,
  `docking_port`) instead of the old `standard.rs` + `ordnance.rs` order. The
  three light hulls now sit with `reinforced_hull_section` instead of after the
  controller. No id changed and no retained section's body changed; only the
  order of entries inside `sections/base.content.ron`.

## Magic strings left as literals

The task's rule is "repeated local runtime IDs use private local constants".
These repeats are deliberately NOT constantized, because the repeated text sits
in more than one namespace and one constant would assert a relation that does
not hold:

- `lessons.rs`: `combat_stance`, `novaos_rebind_section` and the other paired
  repeats are a lesson id AND a media clip stem AND, for some, an input action
  name. Three namespaces that are free to diverge.
- `styles.rs`: each greeble kit id sits next to its own `greeble(id)` asset
  path in one struct literal. The per-kit namespacing tests already pin id to
  path, so a constant would add nothing a rename could not break anyway.
- `balance.rs`, `content_report.rs`, `lint_walk.rs`: `player_spaceship`,
  `example`, `art`, `consumer` and friends are synthetic ids inside `mod tests`
  fixtures. The policy keeps isolated synthetic test ids as strings.
- `gauntlet.rs` names `torpedo_section` once, as a one-use authored reference.

## Shared ids promoted to `nova_ship`

`nova_wfc::plan` is runtime code that cannot reach `nova_authoring`, and it
named two prototypes by literal. Both are now in
`nova_ship::sections::catalog_ids`, which is the lowest crate both sides see:

- `PDC_PIERCE_TURRET_SECTION_ID`
- `TORPEDO_SECTION_ID`

`VECTOR_THRUSTER_SECTION_ID` stays authoring-owned. It is declared beside its
builder in `sections/thruster.rs` and re-exported by `sections` for the one
block hull that mounts it.

## Corrections made while sweeping comments

- `sections/railgun.rs` claimed "two shipped lances"; only one prototype
  remains after the siege railgun was removed.
- `lint_walk.rs` said the crate sits at `crates/nova_assets`; it sits at
  `crates/nova_authoring`.

## Defect found by the migration, and fixed

Moving the two siege weapons off the base catalog and onto the warship fixture
as INLINE sections turned a latent content error into a start refusal.

`kit::dry_magazines` patches every weapon on a flight-behaviour range to
`AmmoCapacity::Limited(0)` and left the section's `Batch` reload alone. While
the siege weapons were catalog prototypes the patch rode on the REFERENCE, and
`lint_section_config` only walks INLINE configs, so nothing checked the result.
Inline, the same patch is linted, and `Limited(0)` under a `Batch` reload is an
error:

    on_load_scenario: refusing to start 'ai_combat_range' (2 content error(s)):
      section 'fixture_siege_torpedo_bay': reload requires a Limited ammunition
      of at least one round
      section 'fixture_siege_railgun_lance': reload requires a Limited
      ammunition of at least one round

`empty_magazine` now patches `reload: Disabled` beside the empty magazine,
which is what the helper's own sentence ("the guns are on the ship and they are
dry") always meant: a batch reload out of an empty rack refills itself. Four
ranges use it - `system_ai_combat`, `system_ai_evade`, `system_ai_patrol` and
`system_helm_orders` - and all four want silent guns.

## Three probe failures left as found

`lesson_combat_reach`, `lesson_combat_moves` and `lesson_advanced_ai_flight`
fail `probe run --correctness-only`. A `sprout` worktree at `fbee23138` with
none of this work applied fails all three the same way, so they are not this
task's (the evidence is in `PROOF.md`).

Two are the harness clock: `sheet_written` is true at once when `NOVA_CAPTURE`
is unset, so a sheet beat ends after N free-running frames rather than N frames
at the capture fps, and the motion the sheet is meant to hold has not happened
yet. Fixing that means changing what a sheet beat WAITS ON across every lesson
that has one - a change to `nova_autopilot` semantics, not to content
ownership. It is left alone rather than folded in here.

## Bench prose moved with the bench fixtures

`docs/agent-bench.md` and `crates/nova_bench/scenarios/README.md` each describe
the five loose fixtures in a sentence. Two of those sentences named content
this task moved: the arsenal's "player warship with point defence, torpedoes,
and two railguns" is now a hull authored inline in the fixture, its
"carrier-sized ram target" is a hauler, and the late hostile in both arsenal
and hunt is a picket. Updated in place; the scenario object ids inside the RON
(`raider`, `pursuer`) are authored names and stay.
