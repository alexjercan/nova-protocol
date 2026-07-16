# Retro: Broadside - the capital-combat vertical slice

- TASK: 20260708-203659
- BRANCH: feature/broadside-vertical-slice (landed f53fa5e8)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES: 1 MAJOR, 2 MINOR, 3 NIT; round 2 APPROVE)

## What went well

- Dogfooding did its job on the first live run: example 19 crashed on the
  loader's eager skybox insert - a latent panic every future mod-shipped sky
  would have hit - and the fix (deferred install) improved the modding
  surface for everyone. No unit test would have found it; the full-app
  example did, immediately.
- Verify-first on the AI constants corrected the plan's torpedo-cadence
  claim (4s -> 10s) BEFORE the arena was authored, and the staging math then
  survived the reviewer's independent re-derivation (726u measured vs ~720u
  claimed).
- The arena_combat/gauntlet test-rig lineage transferred wholesale: seven
  production-faithful behavior tests cost maybe an hour because the rig
  shape (committed-RON include_str, loader-faithful handler registration,
  fired OnDestroyed infos, delivery guards) was established two tasks ago.
- The out-of-context review again earned its tokens (x24 now): it caught a
  claims-vs-data divergence a shared-session eye had no chance on, plus two
  end-state logic holes.
- Honest deferral: feel/balance is recorded as needing hands, not claimed
  as done by a harness.

## What went wrong

- R1.1 (MAJOR, claims-vs-data): a batch of three Edit calls failed on
  read-first; the retry re-applied two from memory and dropped the
  asteroid-field zone-clear Victory - while CHANGELOG and TASK.md kept
  claiming all three. Root cause: retry-by-recall with no post-retry sweep
  of the batch's artifacts; the parity test regenerated faithfully from the
  UNEDITED builder, so everything stayed green around the hole.
- R1.2: the example asserted on an entity that lags the gating resource by
  one frame (PostUpdate write -> next-Update spawn); a wait cushion masked
  the race. Staged on state, but asserted on state that was not in the gate.
- R1.3/R1.5: two handlers were authored act-ungated in an act-structured
  scenario - nobody asked "what happens when this fires after the win?"
  until the fresh reviewer did (a post-victory death flipped the earned
  VICTORY to DEFEAT).

## What to improve next time

- After ANY failed batch of edits, re-verify every member of the batch
  against the artifacts (grep the new text per edit), not just the ones
  remembered; a regenerator downstream will faithfully preserve the hole.
- In staged harnesses, every condition an assert relies on joins the stage
  GATE when it can lag the gating state by frames.
- For act/phase-structured scenarios: walk every handler asking "which acts
  may this fire in?" - terminal states especially; gate by default,
  globality is the deliberate exception.

## Action items

- [x] Ledger: bumped `out-of-context-review-pass` and
      `verify-scripted-edits-applied` (x3 -> Pending promotions); added
      domain lesson `gate-scenario-handlers-to-their-acts` and
      `gate-on-what-you-assert`.
- [x] v0.7.0 plan doc updated: outcome frame + slice marked shipped.
- [ ] Feel/balance playtest of Broadside is the user's; findings become
      `bug`/`balance` tasks at release priority per the plan's policy.
