# Review: Drawer shell + interaction model + objectives section

- TASK: 20260724-102304
- BRANCH: feat/tab-drawer-shell

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/audio.rs:331-332 - the state-route audit
      missed a "while-frozen" mechanism. `pause_loops`/`resume_loops` are wired on
      `OnEnter(PauseStates::Paused)` / `OnExit(PauseStates::Paused)` ONLY, and their
      own comment (audio.rs:328-330, 1058-1060) says audio sinks do not follow
      `Time<Virtual>`, so without them "a loop keeps roaring at its last volume while
      the game is frozen." The Tab drawer freezes `Time<Virtual>` the same way but
      does NOT fire these hooks: if the player is thrusting (or holding RCS) and
      presses Tab, virtual time stops but the thruster hum / RCS hiss keeps playing at
      full volume behind the drawer - the exact defect the comment guards against, on
      the drawer's own headline path. The NOTES.md audit enumerated only the 19
      observer self-guards and did not survey `OnEnter/OnExit(Paused)` freeze hooks, so
      this fell through. Fix: register the same loop pause/resume on
      `OnEnter(PauseStates::Drawer)` / `OnExit(PauseStates::Drawer)` (mirroring the
      `Paused` wiring), or convert the two audio hooks to run on the frozen axis
      generally (e.g. an `OnEnter` for each frozen variant, or a state-scoped system
      keyed on `is_frozen()`). Add a small assertion that opening the drawer pauses a
      `ThrusterLoopSfx` sink so a future frozen variant does not regress it again.
  - Response: Fixed in this round. `audio.rs` now also fires
    `pause_loops`/`resume_loops` on `OnEnter/OnExit(PauseStates::Drawer)`, mirroring
    the `Paused` wiring; the comment and `pause_loops` docs updated to cover both
    frozen overlays, and NOTES.md records the audit-gap lesson (sweep
    `OnEnter/OnExit(<state>)` REGISTRATIONS across all crates, not just `== <state>`
    guards). Re-swept the frozen axis: `audio.rs` was the only gap; the other
    `Paused` OnEnter/OnExit sites are the pause-menu freeze (nova_menu, already
    handled by the drawer's own wiring) and `DespawnOnExit(Paused)` for the pause
    overlay UI (correctly Paused-only). On the suggested sink assertion: a headless
    `AudioSink` test is not feasible (rodio needs a real audio device - this is why
    the original `Paused` loop-freeze also carries no unit test), so the fix is
    verified by parity with the proven `Paused` wiring plus the manual audio check
    batched to the owner. `cargo check -p nova_gameplay` + `cargo fmt --check` clean.

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (verifying the round-1 out-of-context fix)

R1.1 fix confirmed: independently re-derived the finding first (`audio.rs:331-332`
did wire the loop freeze on `OnEnter/OnExit(Paused)` only, and a workspace grep for
`OnEnter/OnExit(PauseStates::Paused)` confirmed `audio.rs` was the sole un-widened
frozen-axis registration outside the pause-menu UI). The fix registers
`pause_loops`/`resume_loops` on the `Drawer` variant too; `cargo check -p
nova_gameplay` and `cargo fmt --check` pass. No new findings introduced by the
change (four `add_systems` lines + comment/doc updates). The MINOR observation about
per-observer test coverage is left to the implementer's discretion (the single
`is_frozen()` helper makes the widen mechanical).

Pending user check (not resolved by this review; batched to flow Finish):
- manual: opening the drawer with Tab in a real run - slides in from the right, the
  game pauses, the cursor appears, objectives show expanded, the tab handle is
  visible when closed, Tab and ESC both close it, and the slide reads well. Includes
  hearing the thruster/RCS loops go silent when the drawer opens (the R1.1 fix).

## Verification

- `git diff master...feat/tab-drawer-shell` read in full (10 code files + docs + task docs).
- `nix develop --command cargo check --workspace --all-targets` -> clean (exit 0). The
  new `PauseStates::Drawer` variant compiles across every exhaustive match / example.
- `nix develop --command cargo fmt --check` -> clean (exit 0).
- New tests all green:
  - nova_gameplay (5): `tab_toggles_drawer_state`,
    `tab_is_inert_while_the_pause_menu_owns_the_freeze`,
    `drawer_exposes_tab_handle_anchor`, `drawer_objectives_section_lists_objectives`,
    `flight_input_inert_while_drawer_open`.
  - nova_menu (2): `entering_drawer_freezes_clocks_frees_cursor_and_shows_no_pause_menu`,
    `escape_closes_the_drawer_to_unpaused`.
  - Spot-checked the `would-it-fail-without-it` bar: `flight_input_inert...` narrows
    to red if the guard is put back to `== Paused` (state would not match, burn would
    move); the freeze test goes red if `OnEnter(Drawer)` is removed; the anchor test
    goes red if `update_tab_anchor` is deleted (stays `None`). Meaningful.
- DoD grep proofs pass: `grep -rn "== .*PauseStates::Paused" crates` returns ONLY the
  one intentional site `nova_menu/src/lib.rs:1004` (`sync_outcome_pause`), which is
  correctly left precise (an outcome can never be live while the drawer is open).
  `grep -ni drawer CHANGELOG.md web/src/wiki/keybinds.md` returns the added lines.
- Guard-widen audit independently re-verified: the 18 observer/flag sites widened to
  `is_frozen()` (player.rs x10, targeting.rs x4, camera_controller.rs x1,
  loader.rs:1037, nova_menu regrab_cursor:1027, nova_debug sync_inspector_cursor:171)
  are correct and behavior-preserving for the pre-existing states. The
  `in_state(Unpaused)` set-gates (plugin.rs:170,174; loader.rs:484,1694; editor:167)
  need no change - `Drawer` is not `Unpaused`. The `Paused`-only pause-menu UI hooks
  (`DespawnOnExit(Paused)` at nova_menu:439,524; `setup_pause_ui`) are correctly NOT
  widened, so the pause menu does not spawn for the drawer.
- Lifecycle: `setup_drawer`/`remove_drawer` spawn on player-ship Add and despawn on
  Remove (a reasonable deviation from the step's "spawn OnEnter(Drawer)" wording,
  since the tab handle must persist while the drawer is closed). Leaving Playing with
  the drawer open is handled by `force_unpause` (OnExit(Playing) -> Unpaused +
  unpause clocks), and the panel entities despawn with the player ship, so the drawer
  does not leak into the menu.
- The `Time<Real>` slide (drawer.rs `drive_drawer_slide`) is the correct call: the bcs
  Tween advances on `Res<Time>` (= `Time<Virtual>`), which the drawer pauses; the
  DECISION.md and NOTES.md justification checks out against the bcs behavior.
- `cargo doc` and the `probe` run were NOT re-run here (expensive); the close-out
  reports both clean/OK. The `manual:` slide-feel item remains PENDING for owner
  acceptance - not resolved by this review.

Notes (not blocking):
- Only `on_flight_burn_input` has a direct per-observer drawer test; the other ~13
  widened observers rely on the shared `is_frozen()` helper and are not individually
  pinned. Acceptable given the mechanical single-helper widen, but worth knowing.
