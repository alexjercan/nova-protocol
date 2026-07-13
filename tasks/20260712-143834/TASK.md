# Shakedown: disable GOTO until the first objective (OBJ_B1) completes

- STATUS: CLOSED
- PRIORITY: 43
- TAGS: v0.5.0, scenario, shakedown, verbs, spike

Spike: tasks/20260712-143551/SPIKE.md

Use the controller verb flags to ship GOTO disabled from scenario start and
re-enable it when the pilot clears the first objective, in
nova_assets/src/scenario/shakedown.rs. Author a `SetControllerVerb(id: ID_PLAYER,
verb: GOTO, enabled: false)` in the opening setup event (next to the
`objective(OBJ_B1, ..)` at shakedown.rs:420-424), and a matching
`SetControllerVerb(.. enabled: true)` in the Beat 1 -> 2 handler
(shakedown.rs:429-446) alongside the existing `complete(OBJ_B1)` and governor
release. Author the initial off-state in the scenario, NOT the shared
`basic_controller_section` catalog entry - the pirate reuses it (sections.rs:56;
shakedown.rs:283,336).

Depends on tasks 20260712-143832 (ControllerVerbs) and 20260712-143833
(SetControllerVerb action). "First objective" = OBJ_B1 per the user; note GOTO is
not actually taught until Beat 4 (OBJ_B4, shakedown.rs:518-521), so keeping it off
until Beat 4 is a defensible stricter alternative the flag makes trivial to
retune - default to the user's words (enable at Beat 1). Confirm the scenario
start event fires before the first player input so the one-frame default-on window
never lets GOTO through (spike open question).
