# Flight objective HUD: minimalist top-right status-bar notification (remove compact panel + tab square; retune reveal; gamepad open)

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.9.0,feature,ui,hud

## Goal

Playtest rework (owner, 2026-07-24): the always-on flight objective surface is
wrong. Replace it with a MINIMALIST top-right status-bar notification.

Owner feedback driving this:
- The old compact objectives panel (top-right text list) is still there - REMOVE
  it (objectives now reveal diegetically + live in the drawer).
- Dislikes the "drawer square" (the tab handle) sitting on the right during play -
  REMOVE it, or at most a tiny "Tab" hint. Preferred: a minimalist top-right
  status-bar notification ("objectives" etc.) that is terse (little text) and
  hints "Tab" to open, with a gamepad alternative.
- The diegetic objective reveal is too big + too centered; wants it a bit SMALLER
  and the vanish animation to translate toward the RIGHT (into this notification).

Scope (direction-level; /plan breaks into steps at pickup):

- Remove the always-on compact objectives panel (`hud/mod.rs` spawn_objectives_panel
  / the bcs ObjectivesPanel treatment) from flight; objectives live in the drawer's
  right panel now (task 20260724-102304 shell) + the diegetic reveal (211520).
- Remove the drawer tab-handle square (`hud/drawer.rs` DrawerTabHandleMarker) from
  the flight view; add a minimalist top-right notification in the status-bar strip
  (near `hud/readout.rs`) - terse (current objective one-liner or a count/icon),
  hinting "Tab". Keep it small; it is a status hint, not a panel.
- Repoint `DrawerTabAnchor` to this notification's screen rect (it is 211520's
  diegetic tuck target - the reveal should now tuck INTO the notification).
- Retune the reveal (`hud/objective_reveal.rs`): smaller card; vanish translates
  toward the notification (right).
- Add a GAMEPAD button to open/close the drawer (Tab has no pad equivalent yet;
  pick a free pad button - the flight rig uses bevy_enhanced_input; check
  reference.rs for a free one). Show the pad hint alongside "Tab".

## Notes

- From the 2026-07-24 playtest of the drawer family (shell 102304 + reveal 211520
  + z-order 121541, all LANDED). Files: hud/mod.rs (compact objectives panel),
  hud/drawer.rs (tab handle + DrawerTabAnchor), hud/objective_reveal.rs (reveal),
  hud/readout.rs (top status strip), input/player.rs + input/reference.rs (rig).
- does-the-old-element-survive: this REMOVES the compact objectives panel and the
  tab-handle square - grep their markers/spawns and the tests that assert them.
