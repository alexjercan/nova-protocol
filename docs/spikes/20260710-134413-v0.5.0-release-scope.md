# Spike: What is in v0.5.0, and in what order?

- DATE: 20260710-134413
- STATUS: RECOMMENDED
- TAGS: spike, release, roadmap

## Question

The backlog after the v0.4.0 release held 31 open tasks, all tagged v0.5.0
and all at priority 0, so nothing encoded what the next release actually is.
This spike answers: which tasks ship in v0.5.0, in what priority order, and
which move to the v0.6.0 backlog? A good answer is a backlog where
`tatr ls --sort priority` reads as the release plan.

## Context

v0.4.0 shipped the combat/feel foundation: torpedo pro-nav guidance, flight
assist, the screen-projected-indicator HUD substrate, targeting with
component fine-lock, first audio, camera juice, and an AI combat behavior
state machine. Almost every open task is direction-level (spiked but not
planned into Steps); 13 spike docs in docs/spikes/ carry the design work.

The user set the release theme directly, with a questionnaire resolving the
scope calls:

1. Physics wells first - the headline feature.
2. HUD UI/UX second - a big step up from the current HUD.
3. Small combat mechanics third - ammo and friends.
4. Modding was initially lowest priority, then explicitly cut from v0.5.0
   entirely; everything not selected moves to v0.6.0 at priority 0.

## Options considered

- **Everything stays v0.5.0, just reorder** - keeps all 31 tasks in the
  release. Rejected: 7 Large + 11 Medium tasks is not one release; the tag
  would stay meaningless.
- **Feature-theme release (chosen)** - v0.5.0 is three themes (wells, HUD,
  combat depth), roughly 10 tasks; docs/chores/modding/objectives defer.
  Matches the user's stated goals and keeps the release coherent.
- **Vertical-slice-first release** - build the capital-combat demo scenario
  and pull in only what it needs. Rejected for now: the slice's own task
  says it lands after objectives exist, which are v0.6.0.

Per-theme scope calls (from the questionnaire):

- **Gravity**: substrate only vs substrate + ORBIT vs + well content.
  Chosen: substrate + ORBIT verb - wells become playable, not just a hazard.
- **HUD**: chosen off-screen indicators, multi-target cycle, diegetic flight
  instruments, and the target inset view. Lock-cue polish and the
  screen-indicator promotion to bevy_common_systems were not selected and
  defer to v0.6.0.
- **Combat**: chosen ammo limits, variable damage by section, and
  weapon/damage-type variety as one combat-depth pass. AI retreat defers.
- **Modding**: none in v0.5.0. RON scenario format, scenario config
  resource, piccolo VM prototype, and both objectives tasks all defer.

## Recommendation

v0.5.0 is 10 tasks, priorities encoding the build order (higher = sooner):

| Priority | Task | Title |
|---|---|---|
| 100 | 20260709-193338 | Gravity wells substrate (SOI, one-way gravity) |
| 95 | 20260709-125640 | Residual roll after autopilot release (bcs PD bug) |
| 90 | 20260709-193339 | ORBIT autopilot verb |
| 80 | 20260708-165704 | Off-screen target/threat edge indicators |
| 75 | 20260708-165705 | Multi-target tracking + subtarget cycle HUD |
| 70 | 20260709-103454 | Diegetic flight instruments |
| 65 | 20260710-104421 | Target inset view (RTT probe first) |
| 60 | 20260525-133025 | Ammo limit logic |
| 55 | 20260525-133004 | Variable damage by section |
| 50 | 20260708-162005 | Weapon and damage-type variety (alt-fire, AP/EMP) |

Ordering rationale:

- The PD roll bug sits between the two gravity tasks because ORBIT's
  insertion/station-keeping flies through the same bcs PD controller that
  currently cannot damp fast roll; fixing it first de-risks ORBIT.
- Diegetic flight instruments follow ORBIT so the instrument panel can show
  the new verb's phases from day one.
- Target inset view is last of the HUD block because its first step is a
  feasibility probe (RTT + PostProcessingCamera + wasm on Bevy 0.19) with a
  schematic-panel fallback.
- The three combat tasks are planned as one combat-depth pass (ammo,
  per-section damage scaling, damage types/resistances touch the same
  weapon/health code); ammo goes first as the smallest self-contained slice.

The other 21 tasks are retagged v0.6.0 at priority 0 (backlog): all docs and
chores, the SFX integration test, lock-cue polish, screen-indicator
promotion to bcs, AI retreat, ship editor polish, wasm particles, skybox
action, modding event-handler lookup, the whole modding/objectives arc (RON
format, config resource, piccolo VM, hardcoded objectives, objectives HUD),
and the capital-combat vertical slice.

## Open questions

- Whether the target inset view survives its RTT feasibility probe or falls
  back to the schematic panel - resolved by the probe step when the task is
  picked up.
- Whether the PD roll bug is in the clamp (`normalize(P + D) * max_torque`
  starving the roll axis) or the inertia-frame handling - needs the minimal
  avian repro the task already calls for, in bevy-common-systems.
- Weapon/damage-type variety is Large and last in line; if the release runs
  long it is the natural cut line - it degrades to v0.6.0 without breaking
  either gravity or HUD arcs.

## Next steps

No new tasks seeded - all 10 release tasks already exist; this spike sets
their priorities and defers the rest. Each v0.5.0 task is still
direction-level and gets its Steps via /plan when picked up (the gravity,
HUD, and combat tasks each reference their own design spike doc).
