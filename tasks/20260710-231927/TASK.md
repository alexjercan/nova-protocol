# Keybind hint icons and key remapping (Arma Reforger look)

- STATUS: OPEN
- PRIORITY: 20
- TAGS: backlog,hud,ux,input


## Goal

Playtest (user, 2026-07-10, backlog polish): replace the bracket-text
keybind hints ("[O] ORBIT") with real key/button icons, Arma
Reforger-style, and support key remapping. The hint resolver already
reads live Bindings (input/player.rs FlightVerbHints), so remapping is
mostly UI + persistence on top of what exists.

## Notes

- Explicitly backlog per the user ("polishing for later stage").
- Icon set needs keyboard + gamepad variants; the resolver already
  distinguishes them.
- Remapping wants a settings screen and persistence - scope that as its
  own plan when picked up.
- 2026-07-13 (deliberate-radar spike 20260713-082207, decision D6): the new
  radar gestures (hold/tap CTRL, RMB raise) join the remap surface, and the
  GAMEPAD story needs rethinking here - no free pad input fits hold+tap radar
  (LB = FreeLook, LT2 = combat, DPadUp takes the thumb off the stick). The
  candidate remedy is a press-TOGGLE radar on pad (press on / press commit /
  long-press clear) or moving FreeLook off LB. Do not stress pad ergonomics in
  the radar family itself; solve it here with remapping.
