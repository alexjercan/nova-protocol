# HUD: add NOVA CRT star mark icon to the Computer/TAB status-bar item

- PRIORITY: 40
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

Playtest feedback: the main HUD status-bar item that advertises the Computer
(the "TAB" affordance, which opens the NOVA OS drawer) should carry the NOVA
CRT star mark as an icon, matching the drawer's brand mark.

Code: `crates/nova_gameplay/src/hud/objective_hint.rs` - the status-bar item
`ObjectiveHintItem` spawns a count + a plain "TAB" text (~99-111). The star
mark asset already exists and is used on the drawer brand plate:
`assets/icons/nova_crt_mark.png` (see `nova_os.rs:3501` ImageNode). Reuse it.

## Story

Add the NOVA CRT star mark (the same `icons/nova_crt_mark.png` used on the
drawer plate) as a small icon on the Computer/TAB status-bar item, so the top
bar visually ties the "TAB" affordance to the NOVA OS computer.

## Steps

- [x] Add a small `ImageNode` (the star mark) to the `ObjectiveHintItem` row,
      sized to sit flush with the count + "TAB" text. Done: 15px
      `icons/nova_crt_mark.png` (the same brand mark the drawer plate uses), left
      NATIVE-coloured like the plate (no tint), guarded on the `AssetServer` so
      headless rigs still spawn the count + TAB.
- [x] Place it leading the item so the row reads "[star] [count] TAB"; keep the
      collapse-when-no-objectives behavior intact (the icon is a child of the
      hint item, so it collapses with the `Display::None` toggle on the parent).

## Definition of Done

- The Computer/TAB status-bar item shows the star mark icon flush with its
      text, and still collapses when there are no objectives. (manual: owner
      confirms the star appears on the top-bar TAB item)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay -- objective_hint)
      [The template's `drawer` filter matches 0 tests; these live under
      `hud::objective_hint::tests::*`.]

## Close-out

What changed and why:
- The `ObjectiveHintItem` status-bar block (the count + "TAB" affordance that
  opens the NOVA OS) now leads with a 15px `icons/nova_crt_mark.png` star -
  the SAME brand mark the drawer plate renders - so the top-bar TAB item visually
  reads as "the NOVA OS computer". Left native-coloured (no tint), matching the
  plate. The icon is a child of the hint item, so the existing
  collapse-when-no-objectives `Display::None` toggle on the parent hides it too.
- The icon spawn is guarded on `Option<Res<AssetServer>>` (like the drawer
  plate), so headless rigs without an AssetServer still spawn the count + TAB.
- Updated the module doc: it previously said "renders as plain text (no
  pill/glyph)", now stale since the star glyph was added.

Difficulties:
- A borrow-checker slip in the new test: holding `&ChildOf` from a `single()`
  across the next `world_mut()` query. Fixed by mapping the query result to owned
  `Entity` values (`child_of.0`) before the next borrow.

Self-reflection: reused the existing asset + the drawer-plate ImageNode pattern
rather than inventing anything, so this was a small, low-risk change. Remembering
to fix the "no glyph" module doc is the doc-surface-sweep discipline paying off.
