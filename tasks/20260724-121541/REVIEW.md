# Review: Tab drawer z-order

- TASK: 20260724-121541
- BRANCH: fix/drawer-zorder

## Round 1

- VERDICT: APPROVE
- REVIEWER: in-session (trivial diff - 2 `GlobalZIndex` components + 2 constants
  + one mirror-test; the review skill's trivial-diff carve-out)

### Verification

- Read the full `git diff master...fix/drawer-zorder`: `GlobalZIndex(10)` on the
  backdrop, `GlobalZIndex(11)` on the panel, two named+commented constants, and
  `drawer_renders_above_the_hud`. Nothing else touched.
- Ran `cargo test -p nova_gameplay --lib -- drawer::` -> 5/5 pass including the
  new test; `cargo check --workspace --all-targets` + `cargo fmt --check` clean.
- Correctness: panel z (11) > backdrop z (10) > HUD chrome (0), so the panel sits
  above its own dim backdrop and both above the flight HUD - exactly the reported
  fix. The z values match nova_menu's pause-overlay modal tier; the drawer and the
  pause/outcome overlays are mutually exclusive (`PauseStates` variants +
  outcome-forces-Paused), so sharing the tier cannot cause a same-frame collision.
- No interaction with the drawer's other systems: `drive_drawer_slide` /
  `update_tab_anchor` / visibility do not read or write `GlobalZIndex`, so the
  slide + state-driven visibility are unaffected. The tab handle correctly keeps
  the HUD z (chrome). The diegetic reveal (task 211520, z=0) never coexists with
  an open drawer (it plays unpaused), so no ordering conflict.
- Test quality: fail-first by construction - before the fix the entities carried
  no `GlobalZIndex`, so `.single().expect(...)` on the empty query panics. Mirrors
  nova_menu's overlay-z assertion (`lib.rs:4939`). It pins the CONTRACT
  (component present + ordering), not the pixel stacking - the latter is the
  owner's manual re-playtest, correctly the `manual:` DoD.
- Docs: no surface to update (the task's own note is right - this is pre-release
  polish of the still-Unreleased drawer, and hud.md carries no z-order prose).

No findings.

Pending user check (batched to flow Finish):
- manual: the owner opens the drawer in a real run and the panel sits ON TOP of
  the compact objectives panel + the rest of the flight HUD (the reported bug is
  gone; transparency + slide still read well).
