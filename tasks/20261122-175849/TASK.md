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
  earlier on a Jobs-pane label that truncates at 1024 px. Owner chose A below.
- Option A: `stand_the_cards_on_the_column_floor` sets the comms panel bottom to
  `column_floor_px` (`crates/nova_hud/src/comms_panel.rs:318`), the floor the
  edge-pinned chips and the pickup tick use: max(dock top, notice top 98 px) +
  8 px. `COMMS_DOCK_GAP_PX` is gone; the TODO in `hud_notice.rs` is removed.
- Proof: `a_comms_card_stands_over_the_notice_box` (1280 and 1024 px, 100 % and
  150 % scale, long notice). On the old floor it fails at 1280: card bottom 651
  vs notice top 622.
- Frames `notice-{1920x1080,1280x720}-comms-{phosphor,hardware}-A.png`: card
  bottom border to notice top border is 8 px at both sizes and themes; notice
  centre 0 px off the midline.

## Decision (owner 2026-11-22)
- Comms overlap: option A, "go with A". The comms panel floor sits above
  the notice top, so comms cards always sit about 40 px higher and never
  move when a notice comes or goes.

## Owner check (2026-11-22, option A)
- Owner: "this one also looks good you can close it". Closed.

## Follow-up (2026-11-22): comms cards over the edge-pinned job chip
- Owner: messages with a speaker ("Station Control") must not overlap the
  "JOB-1" directional label (the objective marker) when both sit in the
  bottom-left corner. Owner idea: more padding on the speaker messages.
- Likely from option A: comms cards now stand on the same column floor as
  the edge-pinned chips (build note: both start at 106 px).

## Follow-up (2026-11-22): smaller keybind dock
- Owner: "make the hotkeys at the bottom smaller like maybe half the size
  they are now would be ok". Same bottom-centre layout as the notice and the
  comms column, so it lands with the comms and job chip fix.

## Follow-up (2026-11-22): floating labels keep clear of the HUD
- Owner: "try to keep the UI not overlap with eachother, especially the
  objective labels which are floating, they sometimes overlap with things,
  let's try to keep the UI clean".
- Rule: a floating objective label (on screen or edge-pinned) never draws over
  a fixed HUD panel, and floating labels do not stack on each other.

## Follow-up (2026-11-22): objective chips on enemy ships
- Owner: "maybe don't show the neutralize chip for objective for too long on
  the enemy ships because it adds a lot of UI and makes it hard to see; or
  like figure out a way to make these floating chips for objectives a bit
  better". Design first: options with frames, owner picks.

## Design (2026-11-22, objective chips)
- Page: `DESIGN-objective-chips.html` (before frames in `frames/`, raw in
  `~/Pictures/nova-objective-chips/`). Proposal only; the owner picks.
- Today: every `ObjectiveMarkerTarget` body wears a full gold chip (diamond,
  `JOB-n Neutralize`, range) at every range, on screen or edge-pinned, until
  the tag leaves (`crates/nova_hud/src/objective_markers.rs:87`). No cap. In
  the frames each chip covers its own red triangle and is wider than a raider
  at 270 m.
- Options:
  - A. Settle: full chip for 4 s after the tag lands, then the diamond only.
  - B. Focus: full chip on the locked or nearest body, diamonds on the rest.
  - C. Edge leads: full chip only while edge-pinned; diamonds on screen.
  - D. Group: one count chip per verb on the nearest body.
- Agent recommendation: C + A. Full chip off screen; on screen the chip
  rides in for 4 s (the HUD notice dwell) when the target comes into view
  after 1 s or more off screen, or the tag lands or changes, then settles to
  the diamond. Lands after the keep-out work.

## Audit (2026-11-22, keep-clear)
- Fixed panels: keybind dock (`keybind_dock.rs:333`), HUD notice
  (`hud_notice.rs:220`), pickup tick (`pickup_tick.rs:64`), comms stack
  (`comms_panel.rs:283`), objective stack (`objective_stack.rs:240`), readout
  strip (`readout.rs:142`), target inset (`target_inset.rs:374`), cinematic
  prompt (`cinematic_prompt.rs:82`) and title (`cinematic_title.rs:136`),
  `nova_ui` status bar (`status_bar.rs:31`).
- Floating labels: flight chips (`flight_status.rs:211`), maneuver readouts
  (`maneuver_instruments.rs:130`), GOTO/ORBIT cues (`keybind_dock.rs:496`),
  edge arrows (`edge_indicators.rs:105`), beacon, objective, waypoint and
  cargo chips (`beacon_chips.rs:71`, `objective_markers.rs:93`,
  `waypoint_chip.rs:62`, `cargo_canister_chips.rs:99`).
- Graphics that stay on their target and never move: reticles, rings,
  allegiance and section markers, ammo gauges, the destination sprite.
- Before: only the cues and the lock readout cleared the inset; the clamp
  floor (`column_floor_px`) ignored the comms stack, so an edge-pinned chip
  sat on the bottom card; nothing separated labels. Frames
  `notice-*-{1card,3card}-*-comms-chip-before.png`,
  `notice-*-overlap-*-keep-clear-before.png`: JOB-1 pinned over the bottom
  card, JOB-2 over the middle card, JOB-3 over JOB-4.

## Build (2026-11-22, comms and job chip, keep-clear)
- Owner: `crates/nova_hud/src/hud_keep_out.rs`. A fixed panel carries
  `HudKeepOutPanel`; `HudKeepOuts::rects` is the union of the laid-out panels,
  the notice rect and the inset rect. The status bar gets the marker as a
  required component.
- Label pass (`screen_indicator.rs`, `update_screen_indicators`): labels carry
  `ScreenLabelType` (`:165`) and are placed in type order; each is fitted with
  `fit_box` (`:428`) against the panels and the labels already placed, 8 px
  apart (`LABEL_GAP_PX`). An edge-pinned label also clears the bottom-centre
  column (`column_top_px`, `keybind_dock.rs:124`), so it does not jump when a
  notice comes. A label moved on screen shows no edge arrow; a pinned one keeps
  the clamp's arrow. The flight chips and the lock readout fit against the
  same union.
- Deleted: `clear_cues_of_notice` and `CUE_NOTICE_GAP_PX` (cues are labels
  now), `clamp_insets`, the floor argument of `place`/`clamp_*`.
- Proof: `an_edge_pinned_chip_never_overlaps_a_comms_card` (failed before,
  passes), `a_label_over_a_panel_moves_clear_without_an_edge_arrow`,
  `a_later_label_gives_way_to_an_earlier_one` (both arrows hidden),
  `a_box_between_two_keep_outs_clears_both_by_the_shortest_path`.
  `cargo test -p nova_hud --lib`: 344 pass.
- Frames (`~/Pictures/nova-hud-notice-centre/`, `*-comms-chip-after.png`,
  `*-keep-clear-after.png`, both themes, 1280x720 and 1920x1080): chip to card,
  label to label gaps 8 to 9 px; no label over a panel.
- Open: a marked hostile behind the player shows two edge cues (gold chip and
  red edge arrow). Cheap fix: hide the edge arrow whose target wears a pinned
  objective chip. Not done; it waits for the objective chip design.

## Build (2026-11-22, smaller keybind dock)
- Chip padding and chip gap go from 8 / 7 px to `metrics::GAP_S`
  (`keybind_dock.rs:141`, `:145`); a portrait keycap (the mouse) is held to the
  row height (`KeyCap::row_size`, `key_glyphs.rs:327`). Dock chips 45 -> 32 px
  tall at 1280x720 and 1920x1080, both themes.
- Stopped at about 71 %, not half: the smallest glyphs were already at the
  readable floor (label caps 8 px, keycap legends 9 px, measured before and
  after), so text and keycap height stay.
- Everything on the dock top follows `column_top_px`/`column_floor_px`.
