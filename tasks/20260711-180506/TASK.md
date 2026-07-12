# Starter New Game scenario: fun but gentle

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.5.0,scenario,content,spike

Goal: the scenario New Game actually drops you into. More fun than the
03_scenario asteroid field but still simple: enemies allowed, but not
aggressive and not instantly lethal (e.g. passive until provoked, gentle
damage). The design spike is done: build the "Shakedown Run" scenario
(id shakedown_run) per the beat sheet in the spike doc - five beats
(burn to beacon, freelook find, salvage sweep, GOTO/ORBIT hands-off,
then a pirate that spawned in the debris cluster during hands-off as the
finale), planetoid + belt setting, objectives gated by trigger areas -
then swap New Game to load it. Legs are short (a few hundred meters
between objectives) and ships minimal (one turret each); objective
conveyance is layer 0 per the spike (imperative text with [KEY] names,
emissive blinking props, short distances) so the scenario works before
the conveyance visuals task (20260712-093831) lands and upgrades in
place after.

Notes:
- Spike (design, beat sheet): docs/spikes/20260712-092926-starter-scenario.md
- Spike (parent direction): docs/spikes/20260711-180500-main-menu.md
- Parent task: 20260711-174915
- Depends on: 20260711-180426 (New Game wiring; swap the scenario id here)
- Depends on: 20260712-093044 (nav beacon + salvage crate objects)

