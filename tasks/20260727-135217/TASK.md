# HUD: add NOVA CRT star mark icon to the Computer/TAB status-bar item

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.9.0,feature,ui,hud

Playtest feedback: the main HUD status-bar item that advertises the Computer
(the "TAB" affordance, which opens the NOVA OS drawer) should carry the NOVA
CRT star mark as an icon, matching the drawer's brand mark.

Code: `crates/nova_gameplay/src/hud/objective_hint.rs` - the status-bar item
`ObjectiveHintItem` spawns a count + a plain "TAB" text (~99-111). The star
mark asset already exists and is used on the drawer brand plate:
`assets/icons/nova_crt_mark.png` (see `nova_os.rs:3501` ImageNode). Reuse it.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Story

Add the NOVA CRT star mark (the same `icons/nova_crt_mark.png` used on the
drawer plate) as a small icon on the Computer/TAB status-bar item, so the top
bar visually ties the "TAB" affordance to the NOVA OS computer.

## Steps

- [ ] Add a small `ImageNode` (the star mark) to the `ObjectiveHintItem` row,
      sized to sit flush with the existing count + "TAB" text (match the bcs
      status-item height/metrics; tint to the status text/muted color if the
      other icons are tinted).
- [ ] Place it consistently (e.g. leading the item) so the row reads
      "[star] [count] TAB"; keep the collapse-when-no-objectives behavior
      intact (the icon collapses with the item).

## Definition of Done

- The Computer/TAB status-bar item shows the star mark icon flush with its
      text, and still collapses when there are no objectives. (manual: owner
      confirms the star appears on the top-bar TAB item)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay drawer)
