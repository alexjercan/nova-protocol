# Scenario timer primitive: elapsed-time events / delayed actions for authored pacing

- STATUS: OPEN
- PRIORITY: 51
- TAGS: spike,v0.7.0,scenario,modding,feature

Goal: the scenario engine has no notion of elapsed time - no delay action,
no timer event (events.rs:13-30, actions.rs:28-55); pacing can only come
from proximity gates or player actions, so authors cannot write "breather,
then reinforcements". Add a small timer primitive so scenarios and mods can
author time-based pacing.

Direction notes:
- Candidate shapes (decide in /plan): an OnTimer event kind
  (id + seconds, one-shot or repeating), or an elapsed-seconds term in the
  filter expression language, or a delayed-action wrapper. Prefer whichever
  composes with the existing variable-gate patterns and fails closed.
- Respect pause semantics (OnUpdate freezes under PauseStates::Paused -
  timers must too).
- Ships with a content proof: use it for a wave breather in the reworked
  second scenarios (tasks/20260717-112630, tasks/20260717-112639 land
  first with proximity gates; this upgrades their pacing).
- Mod-facing surface: plan failure paths up front (docs/LESSONS.md
  mod-facing-surface-plans-failure-paths) and document the RON syntax in
  the same change (author-facing-schema-needs-syntax-doc).

Spike: tasks/20260717-111808/SPIKE.md (finding F5; Options C)
