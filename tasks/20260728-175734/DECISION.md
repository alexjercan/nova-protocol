# DECISION - nova_ui theme + widgets: legacy-palette migration boundary

- STATUS: ACCEPTED
- DATE: 2026-07-29
- TASK: 20260728-175734 (epic 20260728-175719)

## The fork

Step 1 as planned said to DELETE the navy/cyan theme constants
(`BG/PANEL/PANEL_RAISED/BORDER/BORDER_BRIGHT/CYAN/CYAN_BRIGHT/SELECTED_FILL`).
Those constants have ~137 live references across surfaces this task does NOT
own: the flight-HUD chrome (23 refs; restyled by 20260728-175742) and the
editor (24) + menu screens (90) (restyled by 20260728-175738). Deleting the
constants BREAKS THE BUILD unless this task also migrates every one of those
references - which reaches into two sibling tasks' scope. So "delete the
constants" and "stay in this task's scope (theme tokens + widget layer + menu
button fold)" cannot both hold; the compile constraint forces a choice.

(Note: this task's DoD-4 hex grep - `5cc8ff|8fe0ff|141a2e|0b0f1c|183a4e` - is
satisfied regardless, because the consts are written as `srgb_u8(decimal)`, not
hex strings; the 4 grep hits were hex in doc comments, now removed. The
migration is forced by COMPILATION, not by DoD-4.)

## Options (mutually exclusive)

- **A - delete now + migrate all consumers now:** blast the palette swap into
  HUD + editor + menu in this task; the interim look is a rough green auto-map
  that 175742/175738 immediately re-tune, and the edits are throwaway once those
  shape tasks run.
- **B - keep the legacy consts, defer their deletion:** this task ADDS the NOVA
  OS tokens + the skin-aware widget layer and folds the menu button system;
  the legacy navy/cyan consts stay (marked LEGACY, retiring) so HUD/editor keep
  compiling; 175738 and 175742 migrate their OWN surfaces onto the new tokens,
  and whichever lands SECOND deletes the now-unreferenced legacy block.

## Decision (owner, 2026-07-29): B

Keep this task simple and scoped. The legacy navy/cyan block stays in
`theme.rs` under a `LEGACY web palette (retiring)` header that names the two
tasks that migrate + delete it. This keeps the build green, keeps the blast
radius to `nova_ui` + `nova_menu` (this task's real scope), and lets each
sibling task migrate its own surfaces with a real per-screen eyeball rather
than a blind auto-map.

## Consequences / follow-through

- The actual `theme.rs` const deletion is now owned by the sibling tasks:
  - 20260728-175742 migrates the 23 HUD-chrome refs onto NOVA OS tokens.
  - 20260728-175738 migrates the 114 editor + menu refs.
  - The `LEGACY` block is DELETED by whichever of the two lands SECOND (deleting
    earlier breaks the other's build). This is recorded as an explicit step in
    both tasks.
- Live skin reactivity in this task covers `ThemedButton` only (the whole UI is
  mostly buttons). The other shared factories (`panel/panel_head/segmented/
  slider_track/checkbox/list_row/badge`) read the current skin at spawn. Whether
  a live in-place reskin of those non-button widgets is wanted when the Settings
  skin toggle flips is a 175738 decision (that task mounts them in screens);
  KISS default is live-in-place if cheap, else rebuild-on-flip.
