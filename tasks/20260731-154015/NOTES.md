# NOTES - phosphor-only web skin

## What the two skins actually are

`examples/ui/nova_ui_rework_poc.html` defines the light-3D vocabulary in
`:root` and labels it, in its own comment, "(hardware skin)". The phosphor skin
is NOT a colour variant of it - it is a separate block
(`body[data-skin="phosphor"] ...`, the widget zoo) that strips the vocabulary
back out:

| Element | Hardware | Phosphor |
| --- | --- | --- |
| Panel | case gradient + rim + undercut + drop | radial phosphor bloom over a dark green screen gradient, 1px `rgba(54,255,121,0.16)` inset |
| Button | `--face` + rim + undercut + drop, 7px | `rgba(54,255,121,0.05)` + 1px `rgba(54,255,121,0.4)`, 2px, text-shadow glow, `transition: none` |
| Primary | bright key-face gradient | solid `--phosphor`, `#04140a` ink, 14px glow |
| Press | `translateY` into `--well` | fill steps to `rgba(54,255,121,0.2)`; nothing moves |
| Trough | `--well` | `rgba(0,0,0,0.4)` + green hairline |

The landed port (20260731-143918) took the hardware column. This task takes the
phosphor one.

## Token design

The PoC repeats its rgba() literals inline per widget. The site hoists them into
`:root` as a named vocabulary (`--edge*`, `--fill*`, `--panel-face*`,
`--panel-shadow`, `--recess`, `--glow-*`) so a drift is one edit and the parity
test can assert the vocabulary is CONSUMED, not just declared.

The hardware tokens stay defined in `:root`. They are dead there by design: the
`:root` block is a verbatim mirror of the PoC (parity check (a) compares every
token by value), so deleting them would fail the mirror. Check (f) is what keeps
them dead - it scans everything AFTER the `:root` block for a `var()` read of
`--face`, `--rim`, `--undercut`, `--well` or any `--case-*`.

`--drop` is deliberately exempt from check (f): the PoC's own PHOSPHOR panel
shadow ends in `var(--drop)`, so consuming it is faithful, not a leak.

## Deviations from a literal port

1. `--panel-shadow` drops the PoC's `inset 0 0 0 1px` hairline. Every site panel
   already carries a 1px `--edge-faint` BORDER (it did before this task, and
   removing it would resize every panel's padding box), so the inset would draw
   the hairline twice at slightly different radii. The same rule applies to
   recesses: an `inset 0 0 0 1px` is used ONLY where the surface has no full
   border of its own (`.prose blockquote`, `.post-card__ph`). Review round 1
   found five recesses and two dashed placeholders that had both; see REVIEW.md
   R1.3.
2. `--panel-face-hot` has no PoC equivalent - the zoo has no hover state for a
   panel. It is `--panel-face` with the top bloom raised 0.10 -> 0.18 and the
   body lightened one step, so a card hover reads without switching material.
3. `.btn--download` uses an amber build of the same construction. The PoC only
   demonstrates the pattern in red (`.btn.danger`); amber was already the site's
   tertiary accent, so the hue moved and the construction did not.
4. `.prose kbd` and `.prose thead th` have no PoC counterpart on a page. The
   keycap follows `.btn .key` (amber border over a dark trough); the table head
   is a `--fill-hot` strip, NOT a panel - a panel shadow on a 34px cell is all
   vignette and its outer shadows are clipped by the table's `overflow: hidden`.
5. Mermaid needs OPAQUE colours - it derives further shades by colour maths, and
   feeding it the `rgba()` fill tokens left every node grey. `wiki.ts` passes the
   tints already composited over the recess (`#0c1f13` / `#14301c` / `#1d4526`),
   and sets `mainBkg`/`nodeBorder` explicitly: the `dark` base theme resolves
   flowchart node fill from those, NOT from `primaryColor`, so setting only the
   primaries changed nothing on screen. Verified by re-capturing and cropping
   the architecture diagram, not by reading the config.

## Pre-existing quirks left alone (unchanged by this task)

- Mobile 390px: the SCROLL affordance overlaps the LINUX download key.
- Wiki index cards render underlined with amber titles - the whole card is an
  `<a>` inside `.prose`, and `.prose h3` (0,1,1) outranks `.wiki-index__cardtitle`
  (0,1,0).

Both were confirmed pre-existing during 20260731-143918 and are still out of
scope here.
