# Editor: event block corners follow the theme

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.19.0, ui, editor

## User facts
- Owner 2026-11-20: everything square on phosphor; hardware rounds at 2 px;
  the `;` command monitor stays round.
- Not decided: the editor event blocks.

## Agent findings
- The editor canvas event blocks keep their own radius values. Their shape
  shows the block type, so the UI polish pass left them unchanged.

## Delivery
1. Owner rules whether event blocks follow the theme corner (square on
   phosphor, 2 px on hardware) and what then shows the block type.
2. Build it.

## Verification
- Editor frames on both themes.

## Done when
- Event block corners follow the owner's rule.

## Owner check (2026-11-22)
- Keep in v0.19.0 at this priority.

## Proposal (2026-11-22)
- Explainer with frames: `proposal.html` (before frames in `before/`, mock
  frames in `mock/`, full size in `~/Pictures/nova-editor-corners/`).
- Today: the canvas and palette write their own corners
  (`ui/canvas/paint.rs` 10 sites, `ui/canvas/palette.rs` 8 sites). The canvas
  ignores the theme: its area is pixel-identical on phosphor and hardware.
- The corner tells the type for two shapes only: the hat (14 px top) and the
  reporter (round ends). Colour, the `when` word, the missing notch, the
  C-bar and the boolean points already tell every type apart.
- A (recommended): every block box takes the theme corner (0 or 2 px). Type
  stays on colour and silhouette (notch, tab, C-bar, boolean points). Dots
  (fault badge, rail dot) stay round.
- B: A plus a lighter 3 px cap on the hat top and 3 px end bars on a reporter.
- C: phosphor square, hardware keeps today's type shapes. Breaks the
  "hardware 2 px everywhere" rule.
- Owner rules A, B or C.

## Decision (owner 2026-11-22)
- Option A: "A recommeded looks good". Every block box takes the theme
  corner; type stays on colour and silhouette; dots stay round.

## Build (2026-11-22)
- Paint reads the theme: `Paint::live(corner)` / `Paint::picture(corner)`
  (`ui/canvas/paint.rs:70-93`); every block box, chip, socket holder and
  reporter takes `paint.corner`. Hat head and C-foot keep joins square
  (`paint.rs:523-530`). The fault badge stays a dot (`FAULT_DOT`, `paint.rs:63`).
- `sync_canvas` reads `ActiveUiTheme` and redraws every root when the corner
  changes (`ui/canvas/mod.rs:280-301`, stored in `CanvasBoard::corner`).
- Palette pictures draw per entry in `sync_palette_pictures`
  (`ui/canvas/palette.rs:529-553`, `EntryCorner`); the group heading takes
  `ThemedRadius::control()`. The rail dot stays round.
- Died: `RADIUS`, `HAT_RADIUS`, `ENTRY_HAT_RADIUS`, chip 10/4, `...` 4, socket
  10, reporter pill, heading 3, notch/tab 2 literals.
- Proof: `ui::canvas::tests::a_theme_change_repaints_the_blocks_with_the_theme_corner`.
  Before the change it failed (phosphor drew 14, 10, 4, 3, 2 px corners); after,
  all 30 `ui::canvas` tests pass.
- After frames in `after/` and `proposal.html` "After" (full size in
  `~/Pictures/nova-editor-corners/after/`). One walk switches the theme live
  four times; canvas and palette redraw each time.
- Docs: `docs/scenario-system.md` block shapes. Changelog: Interface & HUD.
