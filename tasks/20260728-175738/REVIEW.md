# REVIEW - Menus + editor adopt the reworked widget language (task 20260728-175738)

- ROUND: 1 (out-of-context reviewer, fresh context)
- VERDICT: APPROVE (no MAJOR)
- DATE: 2026-07-29

All three behavior changes verified CORRECT: (a) `ui_skin` threads through
from_resources/load/persist so a flip persists + reloads; (b) `scroll_menu_lists`
drives both mods + scenarios via the shared `ScrollableList` marker and clamps
the stored offset at both ends (byte-for-byte match to the proven
`max_nova_os_scroll_y`); (c) the `apply_paint` try_insert/try_remove fix closes
all four gradient/shadow paths so a same-frame-despawned button no longer errors.
Migration is complete (0 legacy refs in menu+editor), touches no `semantic::*`
accents, no double-substitutions. serde wiring + the required-param sweep are
correct. The 3 new tests are real and tight.

## Findings + resolution

- NIT (robustness): `UiSkin` was only inited transitively via `register`.
  FIXED - explicit `app.init_resource::<UiSkin>()` added next to the other
  defensive inits in the plugin.
- MINOR (contrast): `TEXT_MUTED -> PHOSPHOR_MUTED` (0d6e35) is dimmer than the
  old grey for readable secondary-label text (mod/scenario subtitles,
  descriptions). Legible-but-dim on the dark screen surface. DEFERRED to the
  owner contrast eyeball (DoD 5); it is the phosphor-only interim and no single
  site reads broken. (A `TEXT_DIM` token could be split out if the owner wants
  it brighter.)
- MINOR (scope): "Editor chrome" step - the palette migration DID land
  (card/rail/mod/tooltip); only the section-kind card-tint hand-re-tune is
  deferred. Step is ticked with that note in the Implementation section.
- MINOR (scope): "Screenshots" step - the headless capture-example extension
  (screenshot_ui.rs mods/scenarios/settings beats) + web-capture regen are NOT
  done; folded under the GPU/owner gate (DoD 4). Left unticked, honestly PENDING.
