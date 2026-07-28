# REVIEW - nova_ui theme + widgets (task 20260728-175734)

- ROUND: 1 (out-of-context reviewer, fresh context)
- VERDICT: APPROVE (no MAJOR; MINORs + NITs below)
- DATE: 2026-07-29

The paint model is a clean `(skin, variant, state) -> Paint` pure fn shared by
the interaction observers AND `reconcile_button_skins`, so the two paths cannot
disagree; stale `BackgroundGradient`/`BoxShadow` are correctly removed on a
phosphor flip; the nova_menu fold preserves SFX scope + toggle behavior with no
dead code; the 3 live-tree tests are real and the Added-override test genuinely
depends on the override. Full load-bearing verification (reconciler consistency,
Added-as-system, query disjointness, fold behavior, Bevy 0.19 API, scope
honesty, tests) all PASSED.

## Findings + resolution

- MINOR-1 phosphor hover label used `PHOSPHOR_HI` (#7dffab); PoC is #d6ffe4.
  FIXED (added `PHOSPHOR_HOVER_TEXT`).
- MINOR-2 phosphor selected/primary dropped the PoC glow
  (`0 0 14px rgba(phosphor,.5)`). FIXED (glow_shadow on the inverted branch;
  it is a shadow not a gradient, so the "phosphor is flat" test still holds).
- NIT-1 hardware panel gradient bottom tone/angle (SPACE@180 vs #05080a@168).
  FIXED (CASE_EDGE, 168deg).
- NIT-2 redundant `init_resource::<UiSkin>()` in the example. FIXED (removed).
- MINOR-3 menu button horizontal margin changed (all(8) -> vertical(4)).
  DEFERRED: menu layout is restyled in 175738; noted for its eyeball.
- MINOR-4 disabled+selected renders grey disabled, not dimmed-inverted.
  DEFERRED: edge case (disabled active segmented option); acceptable interim.
- NIT-3 mods checkbox is a visual half-migration (phosphor square, cyan `x`
  from `update_mod_checkbox_labels` + legacy spawn colours). INTENTIONAL interim
  per the code comment; full checkbox restyle is 175738.
