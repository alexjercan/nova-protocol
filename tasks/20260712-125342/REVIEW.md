# Review: Shakedown Run playtest round 3

- TASK: 20260712-125342
- BRANCH: fix/shakedown-playtest-3 (commit 4316ad4 vs master)

## Round 1

- VERDICT: REQUEST_CHANGES (fresh-context agent review)

- [x] R1.1 (MAJOR) objective_feedback.rs - death/quit played the SUCCESS
  chime and spawned green "completed" ghosts for every failed objective:
  teardown clears NovaEventWorld, the write-on-diff sync empties
  GameObjectives, and the snapshot diff read the empty transition as
  "everything completed".
  - Response: fixed - a transition to an empty list is a silent reset
    (snapshot cleared, no cues/ghosts); mid-scenario the list never
    empties (the final handler completes b5 and posts done in one action
    list). Test teardown_to_empty_is_a_silent_reset with a
    real-completion-after-reset delivery guard.
- [x] R1.2 (MINOR) leash had no hysteresis: a hostile parked at the
  boundary ping-pongs Engage/Patrol per crossing.
  - Response: fixed - pure leash_exceeded() uses a state-dependent
    threshold (full radius breaks combat; passive re-engages only inside
    radius * 0.8), unit-tested across the band.
- [x] R1.3 (NIT) leash distance used transform.translation while every
  other AI vector uses live_structure_anchor.
  - Response: fixed - the leash reads own_anchor.
- [x] R1.4 (NIT) ghost stack at top 58% can overlap a tall panel.
  - Response: accepted as-is for shakedown's 1-2 objectives; the
    conveyance task 20260712-093831 owns the full HUD layout pass.

Reviewer verified clean in round 1: all 26 mass-edited call sites
correct; passive-flies-home while still targeted; evade-beyond-leash
sane; two-clocks consistent; Transform present when the root-marker
observer runs on every production path (editor included); ghost stack
tier honored; sounds byte-identical regeneration, only two new files;
shakedown leash numbers cover the patrol + scatter; ASCII + honest
TASK.md.

## Round 2

- VERDICT: APPROVE

Both fixes verified in the files; the teardown guard's test was
MUTATION-TESTED (fails with the guard deleted, file restored clean).
Premise confirmed: no legitimate mid-scenario empty-list path exists
(the final handler completes and posts in one action list), so the
guard costs nothing. Hysteresis arithmetic correct; the patrol loop sits
well inside the re-engage band so the scavenger is never band-locked.
