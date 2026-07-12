# Objective conveyance visuals: markers, item highlight, hint emphasis

- STATUS: CLOSED
- PRIORITY: 35
- TAGS: v0.5.0,scenario,hud,polish

## Goal

The visual language that shows the player WHAT the current objective is,
WHERE it is, and WHICH button does it - as reusable substrate pieces, not
scenario hacks. Design settled by the spike (see Notes): a gold "do this
now" accent; an objective marker chip driven by attach/detach scenario
actions (edge-clamp doubles as the direction arrow); intrinsic item
highlight on salvage crates (emissive pulse + apparent-size bracket);
a color-pulse hint emphasis on the keybind cluster; and verify-only for
objective panel progress (the write-on-diff sync + no-ghost-on-tally
already landed with 20260711-180506). Shakedown Run upgrades in place via
scenario data.

## Steps

- [x] Add `OBJECTIVE_GOLD` (srgba(1.0, 0.85, 0.3, 0.95)) next to NAV_CYAN
      in `crates/nova_gameplay/src/hud/mod.rs`, documented as the fourth
      hue rule (cyan nav / red threat / green own-done / gold do-this-now).
- [x] Add `ObjectiveMarkerTarget { label: String }` component in a new
      `crates/nova_gameplay/src/objective_marker.rs` (mirror beacon.rs;
      NOT named ObjectiveMarker - bevy_common_systems already uses that
      for the panel's text lines). `ItemHighlight` lives in the same file.
- [x] New `crates/nova_gameplay/src/hud/objective_markers.rs` mirroring
      beacon_chips.rs: observers on Add/Remove of ObjectiveMarkerTarget
      spawn/despawn a chip - label + distance text (12 px gold), diamond
      glyph (square border rotated 45 deg via UiTransform, parked left of
      the label), ClampToEdge margin 30 with rotating chevron, offset
      above the mesh, alpha breath (period 1.25 s, 0.7..1.0, one SHARED
      wave across chips - simultaneous markers breathe in unison).
      HudTier::Chrome. Registered in hud/mod.rs. Tests: tag lifecycle,
      death-of-target cleanup, label format, breath band sweep.
- [x] Dedupe rule in beacon_chips.rs (`dedupe_marked_beacon_chips`):
      while a beacon carries ObjectiveMarkerTarget its chip's anchor goes
      None (the widget's established hide channel - Visibility belongs to
      the tier system, so suppression avoids it), write-on-diff; detach
      restores the anchor. Test covers mark/unmark and the sibling.
- [x] Scenario actions in `crates/nova_scenario/src/actions.rs`:
      ObjectiveMarkerAttach { target_id, label } / ObjectiveMarkerDetach
      { target_id }; scoped-only id resolution (the DespawnScenarioObject
      pattern), attach inserts / re-attach relabels / detach removes.
      Detach-on-missing-id is debug-quiet (detach after pickup despawn is
      legitimate script shape); attach-on-missing warns. Unit tests
      beside the despawn ones (scoped-only, relabel, missing-id).
- [x] Item highlight: `ItemHighlight` component (nova_gameplay); salvage
      crates carry it intrinsically from spawn plus a `CrateGlow` emissive
      sine on the render child's material (band 3.0..6.0 replacing the
      static 2.0, period = ITEM_HIGHLIGHT_PULSE_PERIOD_SECS = 1.6 s,
      shared with the HUD bracket so mesh and chip breathe together; NO
      per-entity phase, deliberately unlike beacons). New
      `hud/item_highlights.rs`: observer-driven apparent-size hollow
      brackets (min 28 px, border 1.5 px, crate orange, breathing alpha
      0.55..1.0, offscreen Hide). AS EXECUTED: observer pattern (like
      beacon chips), not the reconcile pattern - the tag is entity-bound,
      so Add/Remove observers are the churn-proof shape. Tests: lifecycle
      via prop despawn, band sweep, glow-moves-and-stays-in-band.
- [x] Hint emphasis: `HintEmphasis` resource (verb-name set, only
      ROW_VERBS admissible - unknown verbs refused with a warning) in
      keybind_hints.rs; `pulse_emphasized_rows` runs after
      update_hint_cluster, lerping the row color toward OBJECTIVE_GOLD
      (1 Hz, max lerp 0.85). AS EXECUTED: gold, not white - the pulse
      target joins the accent language instead of adding a fifth hue.
      Restore-on-change guard so a cleared row cannot stick mid-pulse.
      Scenario actions HintEmphasisSet/Clear go through the queued-command
      drain and no-op with a warning when the resource is absent
      (headless rigs). Scenario teardown (both load and unload paths)
      calls HintEmphasis::clear_all - a leaked emphasis would pulse into
      the next scenario (the state-reset lesson from 20260712-125342);
      loader test drives it through the real UnloadScenario observer.
- [x] Shakedown wiring (data only, no beat-chain changes): OnStart marks
      beacon_1; beat 1->2 unmarks it and marks beacon_2 (attach ordered
      after the spawn in the same handler - action lists execute in
      order); beat 2->3 unmarks beacon_2, marks all three crates
      ("SALVAGE" - each marker dies with its crate, survivors answer
      "which is left"); beat 3->4 marks beacon_3 + emphasizes GOTO; the
      orbit handler clears the emphasis, unmarks beacon_3, marks the
      pirate ("SCAVENGER"); done defensively unmarks the pirate. Tests:
      the referenced-ids cross-check now covers marker targets; a new
      config-shape test pins the whole hand-off map, attach-after-spawn
      ordering and the emphasis set/clear pairing; the five-beats walk
      asserts markers and emphasis at every beat through the real
      pipeline.
- [x] Objective progress: no new code; the pinned regression suite
      (a_message_swap_of_the_same_id_leaves_no_ghost,
      teardown_to_empty_is_a_silent_reset) still green. The one-frame
      wholesale text rebuild on change remains for the human playtest to
      judge; escalate to an update-in-place API only on observed flicker.
- [x] Docs: CHANGELOG Added entry; both action pairs documented in
      docs/scenario-system.md; the gold accent rule stated at
      OBJECTIVE_GOLD's definition (where NAV_CYAN's rule lives).
- [x] Verify: cargo fmt --check clean, cargo check --workspace clean,
      all 27 new/touched tests pass (objective_markers 4, item_highlights
      2, keybind_hints 7, beacon_chips 1, actions 8, loader teardown 1,
      salvage 4, shakedown 10 incl. the extended walk); full suite is
      CI's job per project instruction.

Notes:
- Design spike (all constants/choices):
  docs/spikes/20260712-140842-objective-conveyance-visuals.md
- nova_scenario depends on nova_gameplay (Cargo.toml verified), so
  actions may insert nova_gameplay components / mutate its resources -
  the BeaconMarker pattern.
- Beacon chips are HudTier::Chrome (beacon_chips.rs:56); marker follows.
- FlightVerbHints rows: [STOP, GOTO, ORBIT, CANCEL, COMPONENT, TARGET]
  (keybind_hints.rs ROW_VERBS); Alt/RMB/LMB are not addressable - by
  design.
- Open questions parked in the spike: colorblind check and marker-vs-red-
  reticle on the pirate are playtest calls; do not pre-build.
- Spike (design, this task): docs/spikes/20260712-140842-objective-conveyance-visuals.md
- Spike (parent direction): docs/spikes/20260712-092926-starter-scenario.md
  (section "Conveying objectives: layered, degrades to text")
- Enhances: 20260711-180506 (Shakedown Run works without this; each
  piece slots in via scenario data once available)
- Builds on: 20260712-093044 (nav beacon chip proves the indicator
  styling), the screen-indicator substrate, hud/keybind_hints.rs

## Close record

What changed: two new HUD chip modules (objective_markers.rs,
item_highlights.rs) as thin consumers of the screen-indicator widget; the
OBJECTIVE_GOLD accent; the ObjectiveMarkerTarget/ItemHighlight components
(objective_marker.rs, nova_gameplay - the BeaconMarker crate-split); four
new scenario action variants (marker attach/detach, emphasis set/clear);
the beacon-chip dedupe; the crate glow pulse on a shared clock with the
bracket; teardown-clears-emphasis in the loader; the Shakedown Run
attach/emphasis map; CHANGELOG + scenario-system.md.

Alternatives considered (full weighing in the spike): extending beacon
chips with an objective flag (rejected - markers must ride crates and the
pirate too); a 3D world-space marker mesh (new render path, occlusion and
scale problems); an outline shader for the highlight (custom material
work for a polish task; the emissive channel already reads well); a
font-scale pulse for emphasis (flex column reflow shoves the other
rows); an update-in-place objectives API (solves a problem nobody has
observed - the write-on-diff sync landed with the scenario).

Deviations from plan: the emphasis pulse targets gold, not white (stays
inside the four-hue language); item highlight uses Add/Remove observers,
not the reconcile pattern (the tag is entity-bound; reconcile fits
resource-derived sets like target candidates); marker chips share one
breath wave instead of per-chip phase (beat 3 shows three markers at
once - in unison they read as one system, dephased they read as noise).

Difficulties: none structural - every piece had a proven template one
file away (beacon_chips, target_candidates, BeaconBlink, the despawn
action). The one design wrinkle was HOW to hide a deduped beacon chip:
Visibility belongs to the HUD tier system, so suppression goes through
the anchor (the widget's own hide channel, as the verb cues do).
Cross-checkout LSP noise (a parallel session's infinite_ammo field)
looked like compile errors in files this branch never touched; verified
against the worktree's actual structs and ignored.

Self-reflection: the spike carried almost all the design weight - work
was mechanical because the constants, dedupe rule, teardown hazard and
per-beat map were already written down; keeping spike/plan/work as
separate passes paid for itself. What could be better: the visual feel
(gold against the cyan HUD, breath rates, glow band) is still unproven -
this task inherits the human playtest debt from 20260711-180506, and the
spike's open questions (colorblind check, marker-vs-reticle on the
pirate) are playtest calls. The walk test asserts component state, not
pixels; it cannot close that gap.
