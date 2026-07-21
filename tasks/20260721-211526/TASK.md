# Comms messages: notification-style stacking, skip control, speaker icons, dismiss

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,feature,hud,ui

## Goal

Owner direction (playtest, 2026-07-21): make comms messages richer -
STACKING (multiple visible as notification-style popups, new messages
under/on top per the questionnaire), a SKIP control, a per-speaker ICON,
and explicit dismiss (keypress) alongside the timeout; placement stays
left, top-vs-bottom per the questionnaire; the FULL conversation log lives
in the Tab drawer (that part rides the Tab family).

Today's panel (task 20260717-163033) shows ONE line, queued with dwell -
this task grows it into a stack. Release slot per the questionnaire
(v0.9.0 default under the no-new-features rule). /plan breaks it into
steps at pickup.

## Notes

- Depends on: 20260721-211512 (the Tab drawer spike) for the log view; the stack itself is
  independent.
- Owner questionnaire answers (2026-07-21) recorded here when taken:
  placement, stacking direction, dismiss model, release slot.
- Cast icons tie to the cast constants (crates/nova_assets/src/scenario/
  cast.rs) and the Ledger's speakers - mod-authorable icon refs need a
  content-schema thought (spike may cover).
