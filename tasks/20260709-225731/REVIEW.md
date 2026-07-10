# Review: AI evasion under fire: threat model + jink maneuvers

- TASK: 20260709-225731
- BRANCH: feature/ai-evasion (local branch by user request)

## Round 1

- VERDICT: APPROVE

Checked: full diff vs master; TASK.md Goal/Steps/Notes against the code;
targeted test modules re-run green (threat, evade, jink, behavior state,
target selection, patrol/idle, rotation, fire discipline, point defense,
torpedo section - 76 tests); cargo check --workspace --all-targets clean;
fmt clean. Physics-harness modules were not run locally per the standing
instruction (CI runs the full suite); the touched Engage-path logic they
exercise is unchanged and their unit-level siblings pass.

Spec check: all three deliverables land - the threat model (damage memory
via a HealthApplyDamage observer + aiming-at-me proxy), timed jinks off the
pursuit vector decaying back to Engage, and the deferred 225727 items
(source attribution through ProjectileOwner incl. the blast entity, and
recently-damaged-me scoring in pick_ai_target). The refractory cooldown
protecting the standoff orbit from permanent evasion is a sound addition
and is well documented at the constant.

Existing tests were extended (calm() signals, added Time resources), not
weakened; the transition table's old assertions all survive verbatim.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/input/ai.rs:445 - the observer
  is only tested with the event triggered directly on the AI root; the
  production path triggers on the hit SECTION and relies on ChildOf
  propagation to reach the root (the mechanism bcs's glue tests prove, but
  this observer's matching on it is unpinned). Add a threat_tests case that
  triggers HealthApplyDamage on a child section of the AI root and asserts
  the threat records - it pins the observer against the real event path.
  - Response: agreed and added -
    a_hit_on_a_section_propagates_to_the_root_threat (triggers on a child
    section, asserts the root's AIThreat records the shooter). Green.
- [x] R1.2 (NIT) crates/nova_gameplay/src/input/ai.rs:701 - the aim signal
  measures the bearing from the target's root ORIGIN to my anchor, while
  the range gate uses anchor-to-anchor distance. Harmless at these cone
  widths, but using live_structure_anchor for the bearing origin too would
  keep every AI vector on the same convention.
  - Response: fixed - the bearing now runs anchor to anchor
    (live_structure_anchor(t_transform, t_com)), same convention as every
    other AI vector.
- [x] R1.3 (NIT) crates/nova_gameplay/src/input/ai.rs:1057 - Evade has no
  speed budget (the brake regime is bypassed with the standoff envelope),
  so repeated cycles can build speed the Engage re-entry then has to brake
  off. Likely fine - legs alternate and partially cancel - but worth a
  playtest note next to AI_EVADE_SECS.
  - Response: noted in the AI_EVADE_SECS doc comment as a playtest knob;
    no speed cap added (Engage's brake regime catches the overshoot on
    re-entry, and capping mid-jink would mute the burst).
