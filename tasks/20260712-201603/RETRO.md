# Retro: bcs inspector fix upstream + rev bump

- TASK: 20260712-201603
- BRANCH: chore/bcs-inspector-rev-bump (landed as 1fdeb15; bcs 92221ef +
  4a743b2 + 794a886)
- REVIEW ROUNDS: 2 (REQUEST_CHANGES -> APPROVE)

## What went well

- The cross-repo choreography worked exactly as planned at plan time:
  implement + test upstream, verify nova against a temporary [patch] path
  override, get the push approved, swap the override for the rev bump - no
  step blocked on another.
- The reviewer earned its keep AGAIN at a layer the implementer tests
  missed: the isolated-system rig could not see that removing the marker
  leaves the required EguiContext and the hook-inserted multipass schedule
  behind (a duplicate-schedule panic under the real plugin). It read
  bevy_egui's source to find it.
- The MAJOR's fix test armed the REAL component hook with one resource
  insert instead of dragging in the render stack - cheap and faithful.

## What went wrong

- The ported reconcile inherited a latent hazard from the nova workaround
  it copied, and "ported verbatim from proven code" substituted for
  re-deriving what the copied code's removals actually leave behind.
  Proven-in-context is not proven-in-general: nova never exercised the
  rehome path, so the workaround's proof did not cover it.
- A review response claimed a doc note existed before it landed (caught by
  the reviewer's round-2 check); corrected, and the note shipped as a bcs
  follow-up.

## What to improve next time

- When code moves to a more general home (app workaround -> library), walk
  every branch as if new - especially remove/teardown branches, and
  especially component removals: check requires and hooks for what an
  insert brought along that a remove must take away.
- Never write "done" in a Response line for work that is still intended;
  responses are records, not plans.

## Action items

- [x] bcs follow-up doc note (R1.3/R1.4) pushed as 794a886.
- [x] Ledger: bumped `out-of-context-review-pass`; added
      `insert-cluster-must-be-removed-as-a-cluster`.
