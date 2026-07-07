# Retro: Asteroid RigidBody husk lingers after collider child explodes

- TASK: 20260706-212910
- BRANCH: fix/asteroid-husk
- REVIEW ROUNDS: 1

## What went well

- The parent/child split was already understood from the earlier integrity work, so the fix
  went straight to the right seam: capture `ChildOf` at the `Add IntegrityDestroyMarker` edge,
  before the node despawns. No dependency on child lifetime, no re-derivation of the hierarchy.
- Chose mark-then-deferred-despawn from the start rather than despawning inside the observer.
  That sidesteps ordering races against the other destruction observers (explosion fragments,
  node despawn) - the kind of race that would only surface intermittently in the running game.
- The negative test (non-asteroid parent survives) is what makes the `AsteroidMarker` guard
  trustworthy; without it, "despawn the parent on node destroy" would be a latent ship-killer.
- examples_smoke under Xvfb exercised the real destruction path end to end, so APPROVE rests on
  more than the two unit tests.

## What went wrong

- Nothing substantive. One friction point: the first `Write` to TASK.md failed because the
  file had not been Read in this (post-compaction) context. Root cause: the summary carried the
  content but not the "file has been read" harness state. Fix was a quick Read then edit.

## What to improve next time

- After a context compaction, assume no file counts as "Read" for Edit/Write purposes even if
  its content is in the summary - Read before the first mutation instead of eating a failed call.

## Action items

- [x] proposed AGENTS-style note: after compaction, Read a file before the first Edit/Write
      (captured here; too minor for AGENTS.md on its own, revisit if it recurs).
