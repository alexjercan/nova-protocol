# Add SetAllegiance scenario action - flip a ship's allegiance mid-scenario (neutral-until-provoked primitive)

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.8.0, modding, scenario

## Story

Enable provocation-gated aggression for scenarios: add a `SetAllegiance`
scenario action that flips a spawned ship's `Allegiance` component
(Player/Enemy/Neutral) by scenario id at runtime. Today allegiance is written
only at spawn and never changed (investigation under umbrella 20260722-212808,
ch3 stealth feedback): there is no way to wake a Neutral ship. This action is
the missing primitive for "neutral until provoked" encounters (the ch3 stealth
rework 20260723-000320 consumes it) and is generally reusable.

Mirror the existing `SetSpeedCap` / `SetControllerVerb` actions exactly
(config struct + `EventActionConfig` variant + an apply path that finds the
ship by `EntityId` + `ScenarioScopedMarker` and overwrites `Allegiance`).

## Steps

- [ ] Add `SetAllegianceActionConfig { id: String, allegiance: Allegiance }`
      and the `EventActionConfig::SetAllegiance` variant in
      `crates/nova_scenario/src/actions.rs`, following `SetSpeedCap`
      (~actions.rs:1014-1056) and its RON round-trip.
- [ ] Implement the action: resolve the ship by `EntityId` among
      `ScenarioScopedMarker` entities and overwrite its `Allegiance` component
      (re-use the `Allegiance` type from `nova_gameplay::relations`; check the
      import/prelude path). Warn-and-skip if the id is not found (no panic).
- [ ] Unit test (nova_scenario): a scenario spawns a Neutral ship, the action
      flips it to Enemy, and the component reflects it (fail-first: without the
      apply path the component stays Neutral). RON round-trip test like
      `set_skybox_action_round_trips_through_ron`.
- [ ] content lint stays clean; `cargo check` + fmt green. Doc the action in the
      scenario-system wiki action list + guide-author-scenario if it enumerates
      actions (keep-docs-in-sync).

## Definition of Done

- A scenario RON `SetAllegiance((id: "x", allegiance: Enemy))` flips ship x's
  allegiance at runtime; a Neutral ship becomes hostile (and vice-versa).
  (test: nova_scenario unit test drives the flip.)
- RON round-trips; unknown id warns not panics. (test.)
- cargo check + fmt green; action documented in the scenario wiki. (cmd/manual.)

## Notes

Surfaced by ch3 stealth playtest (owner, 2026-07-23): "go dark" reads as stealth
but the channel Magpies always engage. This action + the ch3 content rework
deliver real neutral-until-provoked stealth. Engine addition (not "data-only"),
justified by the feature gap; ~50 lines + test.
