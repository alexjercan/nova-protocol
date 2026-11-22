# HUD notice: centre the box above the keybind dock

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.19.0, hud, ui

## User facts
- Owner 2026-11-22: "now the notification above the keybinds is not
  centered, it's anchor point is left side and it looks like shit, center
  it please".

## Agent findings
- The HUD notice box is placed with its left edge on the screen midline
  (`web/src/wiki/hud.md`, HUD notice bullet; `crates/nova_hud/src/hud_notice.rs`).

## Delivery
- Centre the HUD notice box on the screen midline above the keybind dock,
  for every notice length and both themes.

## Verification
- Asserted layout: the box centre sits on the midline; frames of a short
  and a long notice.

## Done when
- The notice is centred above the keybinds and the owner has seen it.

## Build (2026-11-22)
- Change: `hud_notice_hud` adds `UiTransform::from_translation(Val2::percent(-50.0, 0.0))`
  on the box (`crates/nova_hud/src/hud_notice.rs:236`); `HudNoticeKeepOut::rect`
  centres the keep-out rect (`:284`), so GOTO/ORBIT cues clear the centred box.
- Proof: `a_notice_box_of_any_length_is_centred_on_the_midline` (live taffy rig,
  `UNDOCKED` and `HULL-3 REPAIR STOPPED: NO PLATES` at 1280 and 1024 px wide).
  Without the transform it fails: `UNDOCKED` box 640..710, centre 675 vs 640.
- Frames (role_goal_sandbox with a reverted capture patch, lavapipe, both themes)
  in `~/Pictures/nova-hud-notice-centre/`. Box centre minus midline: 0 px for
  short (70 px), paid (148 px) and long (226 px) lines at 1920x1080 and 1280x720.
- Open: the left-edge placement kept the box clear of the comms panel (16 px +
  48 % wide, card bottom over the dock top, same height as the notice). Centred,
  a notice under a comms card prints over the card's right end: 1920x1080
  overlap x 847..937 (91 px), 1280x720 x 527..629 (103 px, over the card text).
  1024x768 computed only (card edge 507, short box 477..547): the walk stops
  earlier on a Jobs-pane label that truncates at 1024 px. Needs an owner call.

## Decision (owner 2026-11-22)
- Comms overlap: option A, "go with A". The comms panel floor sits above
  the notice top, so comms cards always sit about 40 px higher and never
  move when a notice comes or goes.
