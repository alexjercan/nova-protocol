# Retro: NOVA OS CRT casing + glass depth pass

- TASK: 20260726-193219
- BRANCH: feature/nova-os-casing-glass
- REVIEW ROUNDS: 1 (APPROVE, out-of-context)

See TASK.md / NOTES.md for what changed and why. This is process only.

## What went well

- Verified the load-bearing APIs against the VENDORED bevy source before
  building the tree: the gradient module (`ColorStop`/`LinearGradient`/
  `RadialGradient`/`BackgroundGradient`), `BorderColor` per-side, `Rot2::degrees`,
  `TextShadow`, `Color::NONE`, and the PoC CSS values. That front-loading kept
  the node tree right the first time everywhere except the one thing I did NOT
  re-check (below).
- The AFTER capture earned its keep again: the first glass highlight was a solid
  `BackgroundColor` rectangle, which read as a grey sticky-note card (Bevy UI has
  no blur). A widget-tree test would have shipped it green; the eyeball caught it
  and I swapped to a `RadialGradient` reflection. `render-output-eyeball`.
- Out-of-context review approved in round 1 with only NITs, and independently
  re-ran the DoD proof + re-derived the uniform field-order match - a clean,
  fast round because the work was verified before hand-off.

## What went wrong

- First compile failed on every bundle carrying a standalone `BorderRadius`
  ("not a Bundle"). Root cause: `BorderRadius` is a FIELD of `Node`
  (`Node.border_radius`) in bevy_ui 0.19, not a component - I checked the
  gradient API against source but ASSUMED `BorderRadius` was still a bundle
  component from an older mental model. The fix (move all radii into the Node)
  was mechanical but touched ~10 sites.
- The test rig broke on a second-order dependency: the chin loads a logo image,
  so `spawn_drawer_shell_with_crt` needed `init_asset::<Image>()`. Caught
  immediately by running the targeted suite, not by review.

## What to improve next time

- Before spawning ANY bevy UI property as a bundle component, grep the vendored
  struct to confirm it is a `#[derive(Component)]` and not a `Node`/other-struct
  field. I did this for gradients; the miss was assuming the ones I "already
  knew". Re-check the ones you know too when the engine minor version moved.

## Action items

- [x] Ledger: bump `render-output-eyeball` (+1, hard glass card) and add
  `bevy-ui-property-is-node-field-not-component`.
- No follow-up code task: the reserved controls row is already the seam for
  the dependent task 20260726-214617; the one open NIT is owner-discretion.
