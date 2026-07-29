# RETRO - Menus + editor adopt the reworked widget language

- TASK: 20260728-175738 (epic 20260728-175719)
- DATE: 2026-07-29
- OUTCOME: shipped; review APPROVE round 1; DoD 4/5 (GPU + owner eyeball) pending.

## What shipped

The palette migration of nova_menu + nova_editor onto the NOVA OS tokens (0
legacy refs left in these crates), the Settings `Interface` UI-skin choice with
persistence, main-menu/pause emphasis (primary/danger/key-chip/footer), the
scenarios scroll fix, and a latent-panic fix in the shared reconciler. 3 new
tests; 73 nova_menu + 13 nova_editor lib tests green.

## What went well

- The `button_on_setting::<GraphicsQuality>` row was an exact template for the
  UI-skin row - the vetting agent confirmed it up front, so the Interface
  section + persistence went in with no design churn (Resource-only `UiSkin`
  satisfies the `Component` bound; `#[serde(default)]` keeps old stores loading).
- Copying `max_nova_os_scroll_y` verbatim for the scroll clamp
  (reuse-known-good-stack) meant the both-ends clamp was right first try; the
  test's dy math lined up with the assertion with no fiddling.
- The full nova_menu suite (not just a filtered test) is what surfaced the
  reconciler despawn panic. Running the WHOLE touched crate's suite once, even
  when the change looks orthogonal, caught a real bug the per-feature tests hid.

## What went wrong / difficulties

- A latent bug from 175734 surfaced here: the skin reconciler's `apply_paint`
  used plain `insert`/`remove` for the gradient+shadow, which errors ("Entity
  despawned") when a menu teardown despawns a button the same frame it repaints.
  8 menu tests panicked. Fixed with `try_insert`/`try_remove` (the repo idiom).
  Lesson: a per-frame reconciler that mutates via Commands must use the try_
  forms - it will inevitably race a despawn. This should have been caught in
  175734 by running the full nova_menu suite, not a single filtered test.
- The legacy-const migration was a blanket mechanical map. `TEXT_MUTED ->
  PHOSPHOR_MUTED` collapses a border colour and a readable-text colour onto one
  token, leaving secondary labels dimmer than before. A per-role map (a distinct
  TEXT_DIM) would read better; deferred to the owner eyeball to avoid
  over-engineering the interim.
- Adding `Res<UiSkin>` to `setup_menu_ui`/`setup_pause_ui` is exactly
  new-required-system-param-sweeps-all-rigs: it only worked because every rig
  routes through the plugin (which inits UiSkin via register). Added an explicit
  init for robustness after review.

## Lessons (for the ledger)

- `reconciler-commands-must-be-try-insert` (domain): a per-frame system that
  paints/mutates entities via `Commands` (a mode reconciler, a skin restyler)
  MUST use `try_insert`/`try_remove`, never `insert`/`remove` - it will race a
  same-frame despawn (menu/state teardown) and the plain forms error via the
  fallback handler (which the smoke examples panic on). Caught by running the
  FULL touched-crate suite, not a filtered test. 20260728-175738 (fix), the bug
  shipped in 20260728-175734.
- `run-the-whole-touched-crate-suite-once` (process): before landing a change
  that touches a shared system, run the whole affected crate's test suite once,
  even if the change looks orthogonal to most tests - a filtered run hid the
  reconciler panic that the full 73-test run exposed. Pairs with BCS "never the
  full workspace suite" - this is per-crate, which is affordable.
- `blanket-token-migration-loses-per-role-meaning` (x1): a mechanical
  old->new colour-const map (BORDER + TEXT_MUTED both -> one token) is fine for
  "retire the palette" but collapses roles that had distinct meaning (a border
  vs readable secondary text), leaving some text dimmer/low-contrast. Flag the
  role-collisions for an eyeball; split a distinct token where legibility bites.

## Follow-ups

- 175742 migrates the HUD's 23 legacy refs and (landing second) deletes the
  LEGACY theme block.
- Owner: contrast eyeball on secondary-label text (PHOSPHOR_MUTED); the
  in-engine both-skin screen review (DoD 5); the web-screenshot regen (DoD 4).
- The headless capture-example extension (screenshot_ui mods/scenarios/settings
  beats) remains open under DoD 4.
