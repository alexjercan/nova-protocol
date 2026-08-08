# NOVA OS terminal: render completion ghost under the block caret

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.9.0, feature, ui, hud

Playtest feedback on the just-landed NOVA OS terminal UX. The completion
"ghost" suffix currently renders AFTER the block caret, so the first usable
letter sits one cell to the right of the caret instead of under it.

Reference: the web PoC `examples/ui/nova_os_terminal_poc.html` (ghost span
lines ~1005, `.ghost`/`.caret` CSS ~459-484). Rust code lives in
`crates/nova_gameplay/src/hud/nova_os.rs`: input line + caret + ghost spawn
~3960-3997, ghost text update ~2238-2240; suffix comes from
`prompt_completion_ghost()` in `crates/nova_os/src/terminal.rs:732`.

## Story

The block caret sits over the cell where the next character will land. The
ghost completion should start UNDER that caret (same cell), so the caret
visually highlights the first letter you would accept, and the rest of the
suffix trails to the right. Today the ghost is laid out after the caret, so
the suffix is shifted one cell right of where typing continues.

## Steps

- [x] Trace the input-line layout: typed text, block caret node, and ghost
      node, and confirm the current left-to-right order that pushes the ghost
      past the caret cell. Confirmed: input_wrap was a flex Row
      [before][caret][after][ghost]; the caret was a flex spacer advancing the
      ghost one cell.
- [x] Re-layout so the ghost's first glyph occupies the same cell as the block
      caret. Done (PoC-faithful): the caret is now `PositionType::Absolute`
      with `left` set each rebuild to the typed-text width
      (`chars * font*NOVA_OS_CARET_WIDTH_FRACTION`), so it no longer advances
      the row and sits over the first after-cursor / ghost letter. Blink alpha
      dropped to 0.85 so the letter reads through (PoC `.caret` opacity).
- [x] Keep the caret blink/hidden states and the accept-completion (Tab) path
      working; the accepted text lands where the ghost showed it. The shell
      model (before/after/ghost strings, Tab accept) is unchanged - only the
      caret node's positioning changed - so there is no off-by-one on accept.

## Definition of Done

- With a completion available, the first ghost letter renders in the same
      column as the block caret; accepting it produces no horizontal jump.
      (manual: owner types a partial command and confirms caret sits on the
      first completion letter)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay -- nova_os_block_caret nova_os_inline_completion)
      [The template's `drawer` filter matches 0 tests; these live under
      `hud::nova_os::tests::*`.]

## Close-out

What changed and why:
- The block caret was a flex item BETWEEN the before-cursor text and the
  after/ghost text, so it advanced the row and pushed the completion ghost one
  cell to the right of the cursor. Now the caret is `PositionType::Absolute`
  (top+bottom stretch it to the line height, width the 0.6em block) and a new
  `position_nova_os_block_caret` system sets its `left` to the MEASURED rendered
  width of the before-cursor text (the before-text node's `ComputedNode.size.x`,
  converted physical->logical via `inverse_scale_factor`). So the caret lands
  exactly on the first after-cursor / ghost glyph - mirroring the web PoC, which
  sets `caret.left = measure.offsetWidth` (it MEASURES, it does not assume a
  cell size).
- Blink on-alpha 1.0 -> 0.85 so the letter under the block reads through (a
  text-mode block cursor over a char), matching the PoC `.caret` opacity.

Alternatives considered / rejected:
- Hardcoding the cell step as `chars * font * 0.6`. This was the FIRST
  implementation; review R1.1 (MAJOR) correctly caught that 0.6em is the PoC's
  decorative BLOCK width (`.caret { width: 0.6em }`), NOT the font's glyph
  advance (IosevkaTerm's is narrower), so the caret would drift ~a full cell by
  ~6 typed chars. Replaced with measuring the real text width, which is
  font-agnostic and drift-free. Corrected the misleading constant doc comment.
- Splitting the ghost into first-char + rest and overlaying the caret only on
  the first char. Rejected: duplicates color logic and does not handle the
  mid-text cursor uniformly; measuring handles every cursor position.

Difficulties:
- Review round 1 (MAJOR R1.1): the initial char-count * 0.6em formula drifts for
  longer inputs. Root cause: I trusted a pre-existing doc comment that conflated
  the caret block width with the glyph advance. Fixed by measuring
  `ComputedNode` width like the PoC. The test was also rewritten (R1.2) to stamp
  a synthetic `ComputedNode` (with a 2x scale factor) and assert the caret copies
  the converted logical width - so it now genuinely pins the measure+convert
  wiring, not the same formula it tests.

Self-reflection: I inherited a constant's claim ("font_size * 0.6 is exactly one
cell") without verifying it against the actual font, and the first test was
tautological on exactly that unverified assumption. Lesson: when a layout value
depends on font metrics, MEASURE (ComputedNode) rather than multiply by an
assumed em-fraction - and make the test assert against the measurement, not the
formula.
