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
