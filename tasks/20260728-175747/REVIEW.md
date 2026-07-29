# Review: Contextual HUD - show-by-relevance + grow-in-use + On/Cinematic

- TASK: 20260728-175747
- BRANCH: feat/hud-contextual-emphasis

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) web/src/index.html:261 - the public landing page still
  advertises the retired three-level cycle ("Cycle the HUD from full chrome to
  instruments-only to a clean screen ... every widget knows its tier", plus the
  figure caption at :276 "The HUD at full chrome beside its minimal and
  clean-screen tiers"). The docs Step is ticked and the wiki/README/keybinds
  were all updated, so this is a straight miss: the symbol-level sweep did not
  catch prose that names the levels without naming the symbols. Rewrite the
  body to the contextual pitch (idle cruise near-empty, elements surface with
  their situation, `~` toggles On / Cinematic) and re-caption the figure.
  - Response: fixed - `web/src/index.html` feature row 06 is now
    "Contextual HUD" with the idle-cruise/surface-with-situation/On-Cinematic
    pitch, and the figure caption re-shot as "The contextual HUD in a live
    combat lock beside the clean Cinematic screen." Confirmed independently
    before accepting: the stale prose was there at :260-261 and :275-276.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/mod.rs:367 -
  `cycle_hud_visibility`'s doc still says "no hold gesture (the spike's call:
  three states are at most two presses away)". Two states now; reword.
  - Response: fixed - now "two states, so one press round-trips".
- [x] R1.3 (MINOR) crates/nova_gameplay/src/hud/torpedo_target.rs:218 - the
  lock readout is a CHILD of the reticle, and both carry a `HudEmphasis`, so
  with the safety off AND the trigger down the readout renders at up to
  `1.12 * 1.12 ~ 1.25`, not the 1.12 the Step names. NOTES.md records the
  inheritance as deliberate but nothing pins the composed value. State the
  measured combined peak at the site and make sure it reaches the owner at the
  DoD-4 playtest gate.
  - Response: fixed - added `LOCK_COMPOSED_FIRING_PEAK` (1.2544) next to the
    two constants, documenting that the child readout composes with the
    reticle pulse, why that is deliberate, and what the fix would be if the
    playtest calls it busy. Pinned by the new
    `the_composed_peak_while_firing_is_the_product` test, which also fails if
    the readout is ever re-parented out of the reticle. TASK.md already routes
    the question to the DoD-4 playtest gate.
- [x] R1.4 (NIT) examples/screenshots/screenshot_orbit.rs:201 - comments still
  speak the retired tier vocabulary ("the instrument HUD tier"; "Keep the full
  HUD" and "at full chrome" in screenshot_combat.rs:347,359-360) while the code
  sets `HudVisibility::On`. Reword to "with the HUD on".
  - Response: fixed - screenshot_orbit (module doc + the capture comment) and
    screenshot_combat (:347, :359-360) now say "with the HUD on"; a repo-wide
    grep for "full chrome"/"instrument tier" over crates and examples is clean.
- [x] R1.5 (NIT) crates/nova_gameplay/src/hud/mod.rs:403 - `Ref<HudTier>` is
  bound to `_tier` and never read; `With<HudTier>` states the intent and drops
  the change-detection wrapper. Relatedly `apply_hud_visibility` walks the
  ancestor chain twice per indicator (`ancestor_tier` then `nearest_gate`); one
  walk returning both would do. No measured cost.
  - Response: fixed - the roots query takes `With<HudTier>` as a filter, and
    the two ancestor walks collapsed into one `resolve_chain` returning
    (is-HUD-managed, gate-open). `ancestor_tier` was left dead by that and is
    deleted; the redundant `Option<&HudTier>` fetch went with it.

Verification the reviewer ran (not findings): `cargo check --workspace
--all-targets --features dev` clean; `cargo test -p nova_gameplay --lib --
hud::` 278 passed; `cargo test -p nova_menu --lib` 73 passed; an independent
`cargo run -p nova_probe -- run playable` at the branch commit came back OK
(5/6 measured, `fps_within_baseline` SKIPPED - no baseline, so there is no
genuine before/after fps comparison and NOTES.md says so). Falsifiability
spot-checks: mutating `pop(HINT_POP_SECS)` to `pop(0.0)` failed the hint
pop-settle test, and forcing `ammo_relevant()` to `true` failed the ammo-gate
test - both restored. No existing test was deleted or weakened; the three
migrated level tests assert MORE than the two they replaced. Ordering and
change-detection claims re-derived independently: `screen_indicator` only ever
writes `UiTransform::rotation`, so scale is genuinely a free axis, and
`HudContextGate` is written with `set_if_neq` so the restore branch fires only
on real transitions.

DoD 2's literal wording ("prints 0 hits") is not met and cannot be: 59 of the
60 hits are `MinimalPlugins` in bevy test rigs and the 60th is the deliberate
historical sentence in `HudVisibility`'s doc. The DoD's parenthetical ("counts
recorded here") is satisfied and the counts in TASK.md match the reviewer's
measurement exactly.

Pending user check (not resolved by this review): DoD 4, the owner playtest -
idle cruise is near-empty, the right things surface at the right time, and
Cinematic gives a clean screen. Two questions are queued for that gate: the
always-on allegiance triangles, and the reticle/readout composed motion (R1.3).

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context

All five round-1 findings verified resolved by the reviewer (checkboxes ticked
above on that confirmation). The reviewer re-derived R1.5's refactor rather
than taking it on trust and showed `resolve_chain` visits exactly the old
managed set (`{entity}` plus strict ancestors) and still returns the NEAREST
gate despite walking past it, so the indicator pass hides and leaves alone the
same nodes as before. It also proved both halves are pinned by mutating each
one separately: neutering the gate half fails
`indicators_inherit_a_shut_gate_from_their_layer` alone; neutering the tier
half fails that plus
`indicator_nodes_are_overwritten_every_frame_via_ancestor_tier`. Deleting the
reticle's `HudEmphasis` fails the new R1.3 test. Re-ran: `cargo check
--workspace --all-targets --features dev` clean, `cargo test -p nova_gameplay
--lib -- hud::` 279 passed, `cargo test -p nova_menu --lib` 73 passed.

- [x] R2.1 (NIT) crates/nova_gameplay/src/hud/torpedo_target.rs:889 -
  `the_composed_peak_while_firing_is_the_product` compares the product against
  a constant DEFINED as that product, so only the structural half can fail and
  the documented `1.2544` lives in prose alone. Pin the literal so retuning
  either emphasis cannot leave the doc silently stale.
  - Response: fixed - the test now opens by asserting
    `LOCK_COMPOSED_FIRING_PEAK` is within 1e-6 of the documented 1.2544, so the
    number the playtest would be told is pinned to the constants.

Pending user check (unchanged, not resolved by APPROVE): DoD 4, the owner
playtest. Two questions ride with it - the always-on allegiance triangles, and
whether the documented 1.2544 composed lock motion reads as too busy.
