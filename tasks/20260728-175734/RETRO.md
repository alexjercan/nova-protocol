# RETRO - nova_ui theme + widgets: NOVA OS palette + skin-aware widget set

- TASK: 20260728-175734 (epic 20260728-175719)
- DATE: 2026-07-29
- OUTCOME: shipped; review APPROVE round 1; DoD-2/DoD-5 owner eyeball pending.

## What shipped

The NOVA OS palette tokens + a `UiSkin` (Phosphor default | Hardware) resource +
a skin-aware `ThemedButton` whose visual is a pure `(skin, variant, state) ->
Paint` function applied by BOTH the interaction observers and a live reconciler
(`reconcile_button_skins`), plus the shared widget factories, the nova_menu
button-system fold, typography routed through `UiFont`, and a `widget_zoo`
example. 3 live-tree tests (one proven red-first).

## What went well

- The pure-`Paint`-function design was the right call: one code path computes
  the visual, two callers (observers + reconciler) apply it, so "the hover
  observer and the skin reconciler disagree" is impossible by construction. The
  reviewer specifically called this out as eliminating a bug class.
- Writing the `skin_switch_restyles_spawned_widgets` test and PROVING it red
  (Added-override disabled) before wiring the override caught nothing broken but
  made the DoD-1 "must fail first" claim honest, not asserted. Cheap insurance.
- Surfacing the migration-boundary fork to the owner BEFORE coding (keep legacy
  consts vs. blast-migrate 137 refs) avoided both a huge throwaway diff and a
  broken build; the DECISION.md + the two sibling-task updates make the deferred
  deletion an owned step, not a loose end.

## What went wrong / difficulties

- `TextFont.font` is `FontSource`, not `Handle<Font>` in Bevy 0.19 - the font
  router failed to compile until wrapped in `FontSource::Handle(...)`. Cheap,
  but a reminder to check the field type, not assume `Handle`.
- Bevy 0.19 `BoxShadow` is DROP-only (no inset). The PoC's hardware bevel leans
  on inset rim/undercut/well shadows, which cannot be reproduced; approximated
  with the face gradient + a drop shadow (same approach the NOVA OS casing
  already uses). Documented in-code so a future reader does not "fix" it.
- Local cargo is a NixOS rustup toolchain whose glibc was GC'd; had to run
  everything through `nix develop --command cargo ...`. Worth knowing for the
  next session (see lesson below).
- Scope creep risk was real: deleting the theme consts would have pulled HUD +
  editor restyling (two whole sibling tasks) into this one. Caught by asking.

## Lessons (for the ledger)

- `bevy-textfont-font-is-fontsource-not-handle` (domain): in Bevy 0.19
  `TextFont.font` is a `FontSource` enum, not a `Handle<Font>`; set it with
  `FontSource::Handle(handle)`. A font-routing system that assigns a bare handle
  fails to compile.
- `bevy-boxshadow-is-drop-only-no-inset` (domain): Bevy 0.19 `BoxShadow`/
  `ShadowStyle` has no inset flag - only outset drop shadows. A CSS design that
  relies on inset rim/undercut/well shadows must be approximated with layered
  `BackgroundGradient` + a drop shadow (the NOVA OS casing precedent), not a
  1:1 port.
- `nix-develop-for-cargo-on-this-box` (process): the rustup toolchain binary's
  glibc interpreter is GC'd, so `cargo`/`rustc` are not directly runnable; run
  `nix develop --command cargo ...` (needs `~/.nix-profile/bin` on PATH). Build
  the heavy test binary once with `--no-run` (long timeout) then run it.
- `one-paint-fn-two-callers-beats-two-color-paths` (positive): when an
  interaction observer and a mode reconciler both need to set a widget's visual,
  factor a single pure `(inputs) -> Visual` function and apply it from both,
  rather than duplicating the colour logic - kills the "paths disagree" bug
  class the reviewer flagged as the top risk.

## Follow-ups (already filed)

- 175738 owns migrating editor + menu (114 refs) off the legacy consts + the
  live-reskin decision for non-button widgets (KISS: live if cheap).
- 175742 owns migrating the HUD chrome (23 refs).
- Whichever of 38/42 lands SECOND deletes the LEGACY theme block.
- Deferred review items (menu margin, disabled+selected mapping, mods checkbox
  glyph) carried into 175738's eyeball via REVIEW.md.
