# Spike: How should the objective conveyance visuals look and behave?

- DATE: 20260712-140842
- STATUS: RECOMMENDED
- TAGS: spike, hud, scenario, polish, v0.5.0

## Question

Task 20260712-093831 names four conveyance pieces (objective marker action,
item highlight, hint emphasis, in-place objective progress) but not how they
should look, move, or integrate with the Shakedown Run scenario. What is the
concrete visual/UX design for each piece so the player always knows WHAT the
current objective is, WHERE it is, and WHICH button does it - fun and not
frustrating - and which pieces are even still needed? A good answer is a
per-piece design (colors, shapes, motion, attach/detach rules) plus the
scenario data changes, concrete enough for /plan to break into steps.

## Context

Verified in code this spike (all paths in crates/nova_gameplay and
crates/nova_scenario unless noted):

- **The screen-indicator widget already does the hard parts**
  (hud/screen_indicator.rs): entity/point anchors, Fixed or
  ApparentSize{min_px} sizing (apparent size measured from collider AABBs),
  pixel offsets, and ScreenIndicatorOffscreen::ClampToEdge{margin_px} with a
  descendant ScreenIndicatorArrowMarker that auto-rotates toward the target
  while clamped. Placement runs in PostUpdate after camera sync.
- **Beacon chips are the consumer pattern to copy** (hud/beacon_chips.rs):
  observer spawns a chip when BeaconMarker is added, despawns it on removal;
  chip = 140x16 px label "BEACON 1  420m" (12 px font) in NAV_CYAN with a
  two-bar chevron shown only while edge-clamped; 30 px edge margin; offset
  (0,-28) floats it above the mesh.
- **The HUD has a settled color language** (hud/mod.rs and per-module
  consts): NAV_CYAN srgba(0.3,0.9,1.0,0.9) = flight-computer projections
  (beacons, orbit cues, hint rows when lit); hostile red family = threats
  and locks; green srgba(0.35,0.9,0.55) = own ship; done-green
  srgba(0.4,0.95,0.5) = completed-objective ghosts; crates are bright
  orange srgb(1.0,0.75,0.15). Hint rows unlit = DIM_COLOR
  srgba(0.5,0.55,0.6,0.5).
- **Hint cluster** (hud/keybind_hints.rs): six fixed rows
  [STOP,GOTO,ORBIT,CANCEL,COMPONENT,TARGET], text+color recomputed from the
  FlightVerbHints resource (lit NAV_CYAN when available, else dim). Alt/RMB/
  LMB are NOT cluster rows (named in objective text only). Anchored [G]/[O]
  on-object cues already exist.
- **In-place objective progress mostly landed already** with the scenario
  task (20260711-180506): NovaEventWorld -> GameObjectives sync is
  write-on-diff (nova_scenario/src/world.rs), and objective_feedback.rs
  treats a same-id message swap ("0/3" -> "1/3") as an update - no ghost
  line, no completion cue (there is a test pinning this). The generic panel
  still rebuilds its text children wholesale on change, but only on change,
  which is one frame of identical layout - no visible flicker expected.
- **Scenario side**: EventActionConfig has 9 variants incl. Despawn; actions
  resolve string ids via EntityId + ScenarioScopedMarker; shakedown_run's
  beats reference beacon_1/2/3, crate_1..3, pirate, planetoid. Beacons
  already pulse emissive (period 1.2 s, luminance 8..60, per-entity phase);
  crates are static emissive orange with a slow tumble.
- **Playtest debt**: 20260711-180506 closed with the human visual playtest
  still open (beacon readability, pickup feel, orbit-gate moment). This
  task's changes land on the same screen; the playtest will judge both.

## Options considered

### Piece 1: objective marker

- **A. Scenario action + marker component + chip HUD module (chosen).**
  New EventActionConfig variants ObjectiveMarkerAttach{target_id, label} /
  ObjectiveMarkerDetach{target_id}; the action inserts/removes an
  ObjectiveMarkerTag{label} component on the resolved entity; a HUD module
  (mirror of beacon_chips.rs) observes the component and spawns a chip.
  Pros: rides the proven widget + observer pattern; attaches to ANY
  scenario entity (beacon, crate, pirate); detach = component removal, and
  entity despawn cleans up for free (entity anchors hide on despawn, the
  observer despawns the chip). Cons: one more chip family on screen -
  needs a dedupe rule with beacon chips (below).
- **B. Extend beacon chips with an "objective" flag.** Restyle the existing
  chip when its beacon is the objective. Pros: fewer nodes. Cons: only
  works on beacons - beat 3 (crates) and beat 5 (pirate) need markers too.
  Rejected.
- **C. World-space 3D marker mesh** (floating diamond above the entity).
  Pros: diegetic. Cons: new render path, scale/occlusion problems at
  hundreds of meters, fights the established chip language. Rejected.

### Piece 2: item highlight

- **A. Emissive pulse + apparent-size bracket, Hide off-screen (chosen).**
  Crates get the beacon-style emissive sine (subtler: the crate is close-in
  content, not a landmark) plus a hollow-border bracket chip using
  ScreenIndicatorSize::ApparentSize{min_px} - the exact pattern
  target_candidates.rs already uses for hostile brackets - in crate orange,
  offscreen policy Hide (direction-finding is the objective marker's job;
  edges stay reserved for threats and the active objective). Bracket alpha
  breathes in sync with the emissive pulse so mesh and HUD read as one
  system. Pros: both halves are existing patterns; "tightens close-in"
  falls out of ApparentSize for free. Cons: two treatments to tune
  against each other.
- **B. Outline/rim shader on the mesh.** Pros: the genre-standard look.
  Cons: custom material/shader work in bevy 0.19 for a v0.5.0 polish task;
  the emissive channel already exists and reads well in space lighting.
  Rejected for now.
- **C. Bracket only (no mesh pulse).** Cheapest, but a static orange box in
  a debris field of orange-lit rocks needs motion to pop. Rejected;
  the pulse is a few lines given BeaconBlink exists as the template.
- Attachment: **intrinsic to the object kind** (SalvageCrateMarker spawns
  highlight automatically), not a scenario action - a pickup that does not
  advertise itself is a bug, not a policy. A reusable ItemHighlight
  component keeps it available for future interactables.

### Piece 3: hint emphasis

- **A. Color pulse on the row, driven by a component + scenario action
  (chosen).** HintEmphasis{verb} / clear actions set an emphasis entry; a
  system running after update_hint_cluster() lerps that row's TextColor
  between its computed color and white at ~1 Hz. Emphasis is a spotlight,
  not a state: it does not change availability semantics - an unavailable
  row pulses dim-to-less-dim, still clearly "not yet". Pros: zero new UI
  surface (the user's explicit constraint from the keybind-stack concern);
  works only on cluster verbs, which is exactly beat 4's GOTO/ORBIT.
  Cons: subtle by design; text stays 12 px.
- **B. Scale/font-size pulse.** More attention-grabbing but causes layout
  churn in the flex column (rows shove each other). Rejected.
- **C. A transient "do this" banner/toast.** New UI channel, exactly what
  the starter-scenario spike ruled out. Rejected.

### Piece 4: objective panel progress

- **A. Verify + pin what already landed (chosen).** The write-on-diff sync
  and no-ghost-on-tally behavior shipped with 20260711-180506. Remaining
  work: confirm in the visual playtest that the one-frame wholesale text
  rebuild is imperceptible, and keep the existing regression tests. No new
  API unless the playtest shows flicker.
- **B. Update-in-place API in the objectives panel** (mutate the Text of
  the matching row instead of rebuilding). Correct but currently solves a
  problem nobody has observed; build only if A's playtest fails. Deferred.

### Cross-cutting: do nothing

Ship layer 0/1 as-is. The scenario works, but beat 3 (which crate is
left?), beat 4 (which button?), and beat 5 (where did the pirate go?) all
lean on text alone; this task exists because that is the frustration
budget. Rejected.

## Recommendation

Build A/A/A/A as one coherent visual system with these concrete choices:

**A new "current objective" accent color.** Objective gold, approx
srgba(1.0, 0.85, 0.3, 0.95), as a shared HUD const next to NAV_CYAN. It is
distinct from every existing family (cyan = nav infrastructure, red =
threat, green = own/done, dim grey = unavailable) so "gold = do this now"
becomes a single teachable rule. The marker chip, the hint-emphasis pulse
peak, and the item-highlight bracket all draw from it or harmonize with it
(crate orange is adjacent, which is fine - crates ARE the objective when
highlighted).

**Objective marker chip**: label + distance in gold (e.g. "OBJECTIVE  |
BEACON 1  420m" is too long - use the attach action's label, e.g.
"NEXT  420m" or the entity label restyled), a diamond glyph (square border
node rotated 45 deg, same construction trick as the chevron bars) instead
of the beacon dot language, slow alpha breath (~0.8 Hz between 0.7 and
1.0 - noticeable in peripheral vision, not a strobe), ClampToEdge with the
existing rotating chevron so off-screen it IS the direction arrow.
Dedupe rule: while an entity carries an objective marker, its beacon chip
hides (one entity, one chip - two clamped chips for the same target would
jitter at the screen edge). Marker sits where the beacon chip would
(offset above the mesh), so attach/detach reads as the chip "promoting" to
gold and back.

**Item highlight**: emissive sine on the crate material (period ~1.6 s,
range ~1.5x..3x the current static 2.0 - visible motion, dimmer than
beacons which are 8..60 landmarks) plus an apparent-size hollow bracket
(min_px ~28, border ~1.5 px, crate orange at breathing alpha), offscreen
Hide. Intrinsic to salvage crates via a reusable ItemHighlight component.

**Hint emphasis**: HintEmphasisAttach{verb}/Detach scenario actions ->
resource or per-row component; color lerp toward white at ~1 Hz layered
after the availability coloring. Only cluster verbs are addressable;
that is a feature, not a gap (Alt/RMB stay text-taught by design).

**Objective progress**: no new code expected; keep the pinned tests, judge
the one-frame rebuild in the playtest, escalate to an update-in-place API
only on observed flicker.

**Shakedown Run integration (scenario data only, no beat-chain changes)**:
beat 1 attach marker to beacon_1; beat 2 to beacon_2 (attach fires in the
same handler that spawns it - action order within one handler's list is
fine, spawn precedes attach); beat 3 detach beacon marker, attach to all
three crates (multiple simultaneous markers are allowed; each despawns
with its crate, so the survivors answer "which one is left"); beat 4
detach crate markers (despawn already did most), attach to beacon_3,
HintEmphasisAttach GOTO, and on the orbit gate clear emphasis; beat 5
attach to pirate; done detaches everything. Every attach targets ids the
script already owns.

Why this shape wins: every piece is a thin consumer of substrate that
already exists (widget, observer chips, BeaconBlink, target brackets,
FlightVerbHints), so the task stays a polish task; the gold accent gives
the player one rule to learn instead of four features to discover; and the
degrade story holds - pull any piece and layer 0 text still carries the
scenario.

## Open questions

Playtest verdict 2026-07-12 (user, after 20260712-093831 landed): the
conveyance visuals are approved EXCEPT gold text readability - "the gold
and white make the text not readable". Two mechanisms implicated: the
emphasis pulse's cyan->gold cross-mix passing through a washed
near-white blend, and the marker label breathing its text alpha down.
Fixed by task 20260712-152340 (pure-gold alpha-only emphasis pulse;
label at constant alpha + dark shadow, glyphs carry the breath). Same
round: "beacons have a really big target thingy" - the lock reticle
sizes to the beacon's 70u trigger sensor (the R1.1 bug class on its
remaining consumer); filed separately. The questions below stay open
where not superseded.

- **Gold vs colorblind readability**: gold-vs-cyan and gold-vs-orange
  should survive deuteranopia (both hue pairs differ mainly in blue
  channel; likely fine). Check with a simulator screenshot during the
  playtest; the diamond-vs-dot shape difference is the backstop.
- **Marker on the pirate vs the red lock reticle**: beat 5 puts a gold
  marker on an entity the combat HUD wants to paint red. Expected fine
  (marker floats above, reticle brackets the hull); if it reads as noise,
  detach the marker once the player achieves a lock on the pirate
  (lock state is visible to the HUD side, not the scenario side - do it in
  the marker system, not the script).
- **Exact pulse constants**: periods/ranges above are starting values;
  the human playtest (still owed from 20260711-180506) tunes them.
- **HudTier for the marker**: Instrument (survives Minimal HUD) or Chrome?
  Lean Instrument - "where am I going" is flight information - but decide
  at /plan when reading how beacon chips are tiered.

## Next steps

Direction-level task (already exists; this spike fills in its design):

- tatr 20260712-093831: objective conveyance visuals - implement the four
  pieces per this spike's Recommendation, then wire the Shakedown Run
  attach/detach data. /plan owns the step breakdown.
