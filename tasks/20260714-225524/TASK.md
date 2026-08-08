# Align HUD accents to the exact web cyan/amber (visual QA needed)

- STATUS: CLOSED
- PRIORITY: 15
- TAGS: ui, backlog, wontdo

## Resolution: WONTDO (2026-07-15)

User is happy with the current HUD accent colors and does not want to nudge them
toward the exact web cyan/amber at this time. The HUD palette stays at its current
centralized values in `nova_ui::theme::semantic` (the zero-visual-change result of
task 20260714-214118). No code change; closing as wontdo. Can be reopened later if
tighter web/HUD brand consistency is wanted.

Umbrella: task 20260714-212139. Depends on: 20260714-214118 (HUD centralization).

## Goal (deferred from the HUD centralization)

Task 20260714-214118 centralized the HUD palette into `nova_ui::theme::semantic`
at the HUD's EXACT current values (zero visual change). The nav-cyan
`(0.3,0.9,1.0)` and objective-gold are close to, but not identical to, the web
app's brand `nova_ui::theme::CYAN` (0.36,0.78,1.0) / `AMBER`. This task nudges the
HUD's brand accents (NAV, OBJECTIVE) toward the exact web cyan/amber for tighter
consistency with the web app.

This is a real VISUAL change (it shifts HUD hues), so it is gated on being able to
eyeball a capture - HUD legibility is a visual property and combat colours carry
meaning. Deferred out of 214118 because the machine could not render a reliable
screenshot at the time.

## Sketch (plan when picked up)

- Decide which semantic accents move: NAV -> theme::CYAN? OBJECTIVE -> theme::AMBER?
  Keep THREAT/ALLY/NEUTRAL as-is (those are combat-meaning, not brand).
- Change the values in `nova_ui::theme::semantic` (and update the pin test).
- Capture `10_playable` / `11_hud_range` with `BCS_SHOT` on an idle machine and
  eyeball: nav chips, objective panel, reticle, faction insets - confirm still
  legible and clearly distinct.

## Notes

- `nova_ui::theme::semantic` is the single edit point now (that was the whole point
  of 214118). The pin test `semantic_accents_match_the_original_hud_literals` must
  be updated to the new intended values when this lands.
- Backlog until a clean-machine visual capture is available.
