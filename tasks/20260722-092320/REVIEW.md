# REVIEW: neutralized (combat-dead) ship state

Summary: A correct, faithful implementation - the predicate, the armed-at-spawn guard, the once-only firing, the AI-only combatant switch-off, the 21 scenario siblings, and the docs all check out; targeted tests pass and the scenario audit (armed vs unarmed) is verified from the real prototypes. Only minor test-coverage gaps and one small robustness nit.

Verified facts:
- `cargo check -p nova_gameplay -p nova_events -p nova_scenario` clean.
- `cargo test -p nova_gameplay --lib integrity::neutralize` -> 4 passed.
- `cargo test -p nova_assets --test neutralized_ships` -> 3 passed.
- Scenario coverage matches DECISION.md audit exactly (21 OnNeutralized: 15 enemy + 6 player). Armament resolved from `assets/base/sections/base.content.ron` prototypes: hauler=cargoa (Hull/Thruster/Controller, NO weapon), corvette/raider/picket/pirate=racer (has Turret), gunship/flagship (Turret+Torpedo). derelict is `kind: Asteroid`, not a ship. Every "left destroy-only" target is genuinely unarmed; every mirrored target is genuinely armed. No regression, no wrongly-mirrored friendly.
- Enemy siblings are idempotent (`VariableSet(=1)` + `ObjectiveMarkerDetach`); player siblings mirror the OnDestroyed act-guard and terminal-act write byte-for-byte (final_tally/lifeline set act=3; broadside/broadside_gunship guard act==2 and set act=3; shakedown/asteroid unguarded). Once-semantics hold: neutralize-then-destroy is benign (asserted in the gunship test).
- Torpedo projectiles carry `TorpedoProjectileMarker`, not `TorpedoSectionMarker`, and are not root children, so they are not miscounted as weapon sections. Collider entities are not section-marked, so multi-collider bodies are unaffected. Player ship is `PlayerSpaceshipMarker` (not `AISpaceshipMarker`), so the `is_ai` guard correctly withholds `AINonCombatant` from the player.
- `test_support` wires `NovaIntegrityPlugin` (which now includes `NeutralizePlugin`), so the unit tests run the real system with real `.after(IntegritySystems)` ordering, not a hollow stand-in.

## Findings

[MINOR] neutralize.rs:59 / ordering across observer hops. `SectionInactiveMarker` is not written inside the `IntegritySystems` set - it is inserted by the `on_section_disable` OBSERVER (glue.rs:48), itself triggered by the core `IntegrityDisabledMarker` observer chain (bevy_common_systems plugin.rs). `.after(IntegritySystems)` only orders past `aggregate_ship_health`/`derive_integrity_leaves`; the disable cascade is a separate observer path fired from the damage/aggregate commands. In practice this only ever makes neutralize detect one frame LATE (never early / false), because an absent-or-not-yet-inactive section counts as "working", so it is safe. Worth a one-line comment that the guarantee is "no false neutralize", not "same-frame detection", so a future reader does not tighten the ordering expecting same-frame semantics. No code change required.

[MINOR] neutralize.rs tests / missing negative AINonCombatant assertion. `neutralized_ai_ship_is_taken_out_of_combat` proves an AI ship GETS `AINonCombatant`, but nothing asserts a NON-AI (player) ship neutralized does NOT get it. The `is_ai` guard is the one thing standing between a player-ship neutralize and a spurious `AINonCombatant`; it is currently untested. Suggest a fourth case: spawn an armed ship WITHOUT `AISpaceshipMarker`, drive it to neutralized, assert `NeutralizedMarker` present, `OnNeutralized` fired, and `!contains::<AINonCombatant>()`.

[NIT] neutralize.rs:127 silent no-op when id/type_name absent. If a neutralized root lacks `EntityId`/`EntityTypeName`, the marker + `AINonCombatant` are applied but no event fires (and no debug line). For a shipped scenario ship this cannot happen (the root always carries both, as the tests rely on), so it is correct, but a `trace!`/`debug!` in the else branch would make a future mis-spawned ship diagnosable rather than silently un-neutralized at the scenario layer.

[NIT] Coverage of the raiders and a mid-act player neutralize is only indirect. The nova_assets test drives broadside (corvettes), broadside_gunship (gunship boss + player terminal), and shakedown (unguarded player). lifeline's seven raiders and its act-1-guarded player Defeat are structurally identical to the tested handlers and were verified by static audit, so this is acceptable, but a lifeline case would close the "biggest scenario, most siblings" gap cheaply if desired.

## Non-issues checked and cleared
- Armed-at-spawn guard prevents unarmed-hull false neutralize and the mid-spawn (children-not-yet-attached) false neutralize: confirmed via the `!was_armed`/`continue` path (neutralize.rs:105-110) and the `unarmed_ship_losing_thrusters_is_not_neutralized` test.
- Once-only firing via `Without<NeutralizedMarker>` + never-removed marker: confirmed by the re-fire assertion in `armed_ship..._is_neutralized`.
- Destroyed leaf (despawn) vs disabled (SectionInactiveMarker): both correctly resolve to "not working" (absent child -> `get` Err; disabled -> `!inactive` false).
- `EventConfig` match is exhaustive with the new `OnNeutralized` arm (events.rs:51); prelude re-exports updated; all three wiki surfaces + CHANGELOG updated.

VERDICT: APPROVE

## Round-1 resolution (author, post-approval hardening)

All four findings addressed (test-only + comment + debug-log; no production
behavior change, so no re-review round needed):

- [MINOR ordering]: expanded the `.after(IntegritySystems)` comment to state the
  guarantee is "no false neutralize" (detection may be one frame late), not
  same-frame - so nobody tightens it. (neutralize.rs)
- [MINOR AINonCombatant]: added `neutralized_non_ai_ship_is_not_marked_non_combatant`
  - an armed ship WITHOUT `AISpaceshipMarker` neutralizes, fires OnNeutralized,
  and does NOT gain `AINonCombatant`. (neutralize.rs tests; 5 pass)
- [NIT silent no-op]: added a `debug!` in the else branch when a neutralized root
  lacks EntityId/EntityTypeName, so a mis-spawned ship is diagnosable.
- [NIT raider/lifeline coverage]: added `lifeline_raiders_and_player_neutralize_as_expected`
  - a raider neutralize sets its kill flag, and a live-act player neutralize is a
  terminal Defeat that retries the lane. (neutralized_ships.rs; 4 pass)
