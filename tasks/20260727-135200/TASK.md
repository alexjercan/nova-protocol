# NOVA OS terminal: render completion ghost under the block caret

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.9.0,feature,ui,hud

Playtest feedback on the just-landed NOVA OS terminal UX. The completion
"ghost" suffix currently renders AFTER the block caret, so the first usable
letter sits one cell to the right of the caret instead of under it.

Reference: the web PoC `examples/ui/nova_os_terminal_poc.html` (ghost span
lines ~1005, `.ghost`/`.caret` CSS ~459-484). Rust code lives in
`crates/nova_gameplay/src/hud/nova_os.rs`: input line + caret + ghost spawn
~3960-3997, ghost text update ~2238-2240; suffix comes from
`prompt_completion_ghost()` in `crates/nova_os/src/terminal.rs:732`.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Story

The block caret sits over the cell where the next character will land. The
ghost completion should start UNDER that caret (same cell), so the caret
visually highlights the first letter you would accept, and the rest of the
suffix trails to the right. Today the ghost is laid out after the caret, so
the suffix is shifted one cell right of where typing continues.

## Steps

- [ ] Trace the input-line layout: typed text, block caret node, and ghost
      node, and confirm the current left-to-right order that pushes the ghost
      past the caret cell.
- [ ] Re-layout so the ghost's first glyph occupies the same cell as the block
      caret (caret drawn over the first ghost glyph), e.g. overlap the caret
      and ghost via absolute/negative positioning or by drawing the caret as a
      background/highlight of the first ghost character rather than a separate
      advancing cell.
- [ ] Keep the caret blink/hidden states and the accept-completion (Tab) path
      working; the accepted text must land exactly where the ghost showed it
      (no off-by-one when the suffix is committed).

## Definition of Done

- With a completion available, the first ghost letter renders in the same
      column as the block caret; accepting it produces no horizontal jump.
      (manual: owner types a partial command and confirms caret sits on the
      first completion letter)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay drawer)
