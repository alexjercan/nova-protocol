# Review: Diegetic objective reveal

- TASK: 20260721-211520
- BRANCH: feat/objective-reveal

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

### Verification

- Ran `cargo test -p nova_gameplay --lib -- objective_reveal:: objective_feedback::` under `nix develop`: 6 passed, 0 failed (2 new reveal + 4 objective_feedback incl. the 3 re-pinned). Exit 0.
- Traced the phase machine (`reveal_phase`, objective_reveal.rs:141-157): appear grows scale 0 -> 1.9 and alpha 0 -> 1 over 0.35s at `base`; hold pins 1.9/alpha 1 at `base` for 2.3s; tuck lerps scale 1.9 -> 0.25, alpha 1 -> 0, position `base.lerp(target, t)` over 0.55s. Bounds are sane; smoothstep is clamped. Despawn at `elapsed >= REVEAL_TOTAL_SECS` (3.2s) is checked before writing transforms - correct.
- Anchor `None` fallback (objective_reveal.rs:145): `target = base`, so tuck does `base.lerp(base, t) = base` and the card fades in place. Graceful, no panic. Matches NOTES.
- Gold-ghost removal verified against the tree: `objective_feedback.rs` no longer feeds `added_objectives -> OBJECTIVE_GOLD` into the ghost stack; completions still spawn `ObjectiveGhostLineMarker { base: GHOST_COLOR }` (green) intact. Other `OBJECTIVE_GOLD` consumers (`objective_markers.rs`, `keybind_hints.rs`) are unrelated features and correctly untouched. The reveal itself reuses `OBJECTIVE_GOLD` for its border/text hue - intentional continuity, not a leftover.
- Teardown: `clear_reveals_on_teardown` (run_if `resource_changed::<GameObjectives>`) despawns reveals when the list empties - the same `is_empty()`-on-resource-change idiom `objective_feedback`'s ghost teardown already uses, so the leak class is handled consistently. The `resource_changed`-on-init-frame gotcha is handled in the rig by seeding objectives non-empty first (documented in close-out).
- Both new tests are would-it-fail-without-it: deleting `animate_objective_reveals` leaves the card frozen at spawn and never despawning (fails both slide + despawn asserts); deleting `clear_reveals_on_teardown` leaves the reveal alive after empty (fails the teardown assert).
- The 3 updated objective_feedback tests were re-pinned to the new behavior, not weakened: they now assert zero gold ghosts on a posting AND (in the swap test) assert exactly one `ObjectiveRevealMarker` - a stronger check than the old gold-count.
- Docs: CHANGELOG `[Unreleased] > Interface & HUD` and `web/src/wiki/hud.md` "Comms and objectives" both updated to the new reveal wording; no live "gold flash" prose remains. The one surviving "flashes gold" hit is `web/src/news/0.7.0.md` - a frozen historical release note, correctly left alone.

### Findings

- [x] R1.1 (MINOR) crates/nova_gameplay/src/hud/objective_reveal.rs:39,161 - `REVEAL_APPROX_HEIGHT_PX` (52.0) is a hardcoded nominal used to vertically centre the card on its target; a multi-line wrapped objective (width 360px, font 22px) will be taller than 52px and will land slightly off-centre on the tab handle during the tuck. Suggest either reading the laid-out `ComputedNode`/`Node` size when available, or documenting the single-line assumption in the const doc so a future multi-line objective author knows the centering is approximate.
  - Response: Documented the single-line assumption in the const doc, noting the offset is acceptable because the card shrinks to a point as it arrives (so the mis-centering vanishes with it). Kept the nominal rather than a `ComputedNode` read to avoid a layout-timing dependency for a sub-pixel-at-arrival effect.

- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/objective_reveal.rs:298-317 - the tuck test asserts the card slides in x (base_left 780 -> anchor x 1880) and despawns, but does not assert the y coordinate tracks the anchor (anchor y=300 vs base y=367). The x-slide + despawn already fail-without-the-system, so this is only a completeness nit: add a `node.top` assertion toward the anchor y to pin the full 2D trajectory, or leave a comment that x is a proxy for the whole slide.
  - Response: Added a `node.top` trajectory assertion (`min_top < base_top - 20`) so the test pins both axes of the tuck (right AND up toward the anchor).

- [x] R1.3 (NIT) crates/nova_gameplay/src/hud/objective_reveal.rs:88 - the reveal card spawns as an orphan top-level `Node` (no HUD-root parent). It renders fine and is cleaned up by the ~3.2s lifetime / teardown-on-empty, so this is not a leak in practice, but every other HUD element in the tree parents under a root. Consider a one-line comment noting the deliberate orphan (matches `screen_indicator`?) so it does not read as an oversight.
  - Response: Added a comment at the spawn site explaining the deliberate orphan (absolute screen-px positioning, transient, must not inherit the HUD-visibility cycle - like the drawer).

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (addressing round-1 non-blocking findings)

The round-1 verdict was already APPROVE; the 1 MINOR + 2 NITs were addressed on
the branch (see Responses). Re-derived the load-bearing round-1 claim
independently before adopting the round: the gold-ghost removal - confirmed
`objective_feedback` no longer feeds additions to the ghost stack, completions
still spawn `GHOST_COLOR` ghosts, and no other consumer depends on the removed
gold posting-flash (`grep -rn OBJECTIVE_GOLD crates` -> only the unrelated
markers/keybind-hints + the reveal's own hue). Re-ran the suite after the fixes:
6/6 green, `cargo fmt --check` clean. No new findings.

Pending user check (batched to flow Finish):
- manual: a real run - a new objective appears large + slightly rotated, holds
  ~2-3s, tucks into the drawer tab handle and vanishes; reads well.

- The `manual:` reveal-feel DoD item (a real run: appears large + rotated, holds ~2-3s, tucks into the tab handle, reads well) is not verifiable from the diff and remains PENDING owner acceptance, as the close-out states.
