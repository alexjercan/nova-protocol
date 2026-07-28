# Retro: web easter egg - menu -> HUD -> NOVA OS CRT chain

- TASK: 20260728-185730
- BRANCH: feat/web-easter-egg
- REVIEW ROUNDS: 1 (APPROVE, out-of-context)

Process notes; what/why/verification live in TASK.md + DECISION.md.

## What went well

- Grounded in the real wiring first (webpack CopyPlugin + historyApiFallback,
  `site.ts` `initEasterEgg`, `site.test.ts`) before editing - the change slotted
  into the existing nova-os pattern with no guessing.
- Guarded the canonical-reference edit: the CRT return only activates with
  `?back=`, so `nova_os_terminal_poc.html`'s default (paramless) behaviour is
  byte-for-byte unchanged. A clean way to extend a shared file without a fork,
  recorded as DECISION D3.
- The chromium eyeball earned its keep: it immediately exposed the collapsed
  corner menu after the first immersive attempt, and confirmed the deployed
  routes render (menu immersive, HUD buttons, Settings skin control) - the
  render-output-eyeball habit catching a green-exit-code-but-wrong-output bug.
- Verified the DoD proofs for real (npm test + npm run build) rather than
  asserting them, using the known node-from-nix-store + node_modules-symlink dance.

## What went wrong

- First immersive attempt hid the demo topbar with `display:none`, which removed
  it from the `.app` CSS grid and reflowed the stage up into the empty `auto`
  track - collapsing it to 0 height and HIDING the corner menu that immersive
  mode exists to show. Root cause: forgot that `display:none` on a grid child
  reshuffles the remaining items across the template tracks. Fix: also collapse
  `grid-template-rows` to `1fr`. Lesson `display-none-grid-child-reflows-tracks`.
  Cost was one screenshot + one-line fix because I eyeballed the render.
- Wrote Step 7 implying the UI-skin choice "survives the menu -> HUD -> CRT ->
  back hops" as if it themed all three, but the HUD/CRT PoCs have no skin
  machinery - it only themes the menu. Caught by the reviewer as a MINOR honesty
  gap. Root cause: wrote the step aspirationally without checking the downstream
  PoCs' capability. Reworded.

## What to improve next time

- When hiding a grid/flex child to make a "mode", re-check the container's track
  or flow assumptions - hiding a child is rarely layout-neutral in grid.
- State a component's capability limits in the plan wording (these PoCs are
  single-look; the skin themes the menu only) instead of implying downstream
  effects the code does not deliver.

## Action items

- [x] Bumped `web-tests-need-node-from-flake` to x2 (glob the node version;
  never `git add` the node_modules symlink).
- [x] Added `display-none-grid-child-reflows-tracks` to the ledger.
- [ ] Owner: live `webpack serve` playtest of the full chain, then deploy via
  `/release` (DoD #3/#4, deliberately left to the owner).
