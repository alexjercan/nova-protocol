# RETRO - ch3 overspeed picket provocation (warn-then-trip)

- Task: 20260723-143603 (umbrella 20260723-143503)
- Outcome: CLOSED, APPROVE round 1 (out-of-context reviewer; one NIT, fixed)
- One commit after squash-land

## What went well

- Building on the sibling engine task paid off exactly as planned: because
  `player_speed` was already a proven reserved readout, the entire feature was
  PURE CONTENT (three OnUpdate handlers + a state variable) - no Rust, no new
  engine surface. The producer/consumer split (engine task first, content task
  second) kept each review small and single-purpose.
- The state machine is robust by construction, not by luck: WARN sets
  `speed_warned` to 1 and TRIP needs 2, and the only 1->2 step (REARM) requires
  `player_speed < 7` which contradicts TRIP's `> 8` in the same frame - so no
  same-pulse warn-and-trip is possible regardless of within-pulse write-visibility
  semantics. The reviewer independently traced the real dispatch model and
  reached the same conclusion. Designing so correctness does not depend on
  handler evaluation order is what made it reviewable.
- Followed `seed-helper-drifts-from-source` deliberately: adding `speed_warned`
  to OnStart meant updating BOTH the rig's `armed_app` seed list and the
  `on_start_seeds_...` key pin in the same change - the reviewer confirmed no
  drift.
- Docs sweep was thorough (keep-docs-in-sync): CHANGELOG + bundle version +
  README + wiki version-history + news draft, all from the final diff.

## What went wrong / friction

- The `docs/news-0.8.0-the-ledger.md` ch3 bullet was already STALE before I
  touched it - it still described the pre-stealth-rework "Magpie ambush" that
  task 20260723-000320 had replaced with neutral pickets. An append would have
  left a contradiction next to my accurate sentence, so I rewrote the bullet to
  the current design. A prior task's doc-sync missed this ephemeral draft.
- One NIT from review: a comment clause I inserted into the RON header ran past
  the file's ~72-col comment width (and doubled a paren). Trivial; rewrapped
  post-approval under the trivial-diff carve-out.

## Lessons candidate (for /lessons at Finish)

- `ephemeral-news-draft-drifts-behind-content` (x1): the `docs/news-*.md` release
  drafts are ephemeral and easy to skip in a docs sweep, so they drift behind the
  content they describe - when a chapter's mechanics change, RE-READ the matching
  news bullet against the current RON and rewrite it, do not just append. Found a
  stale pre-stealth-rework "ambush" bullet two tasks later. 20260723-143603.
  (Possibly folds into the existing `keep-docs-in-sync-with-code` promoted rule -
  the news draft is one more surface on its list.)

## What to do differently next time

- When a task changes a chapter's mechanics, grep the news drafts (`docs/news-*.md`)
  as part of the SAME doc sweep as CHANGELOG/README/wiki - they are on the
  keep-docs-in-sync list even though they are ephemeral, because a stale draft
  ships as stale release notes if not caught before the wipe.
