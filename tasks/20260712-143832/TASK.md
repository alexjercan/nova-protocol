# Controller-provided flight verb flags: gate STOP/GOTO/ORBIT on the controller section

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0,input,controller,verbs,spike


Spike: docs/spikes/20260712-143551-controller-provided-verb-flags.md

Make the flight verbs a capability the controller SECTION grants, with per-verb
enable/disable flags living on that section. Add a reflected `ControllerVerbs`
component (default all-on) seeded from new fields on `ControllerSectionConfig`
and inserted by `controller_section(..)` alongside `PDController`
(nova_gameplay/src/sections/controller_section.rs). Fold the flags into BOTH gate
points in nova_gameplay/src/input/player.rs:

- `update_flight_verb_hints` (player.rs:134-239): read the player ship's live
  controller section's flags and AND each verb's `available` with its flag
  (`goto: flyable && verbs.goto && lock.is_some()`, etc).
- the four `on_autopilot_*_input` execution observers (player.rs:684-819): look up
  the ship's live controller section and return early unless the matching flag is
  set - and add the controller-present re-check the observers lack today (only the
  hint pass gates on `flyable` now, so a dark key still fires).

`flyable` stays as the physical controller+thruster presence gate; the flags are
the orthogonal per-verb layer on top. Every verb defaults on, so this task is a
no-op behaviour change until a flag is written - blocks the SetControllerVerb
action and the shakedown task. Decide storage shape (four bools vs a FlightVerb
enum + set/bitflags) and whether CANCEL/Z is flag-gatable (spike recommends
exempt, so a disabled verb can never strand an engaged autopilot). See the spike's
open questions. First in the family; blocks the other two.
