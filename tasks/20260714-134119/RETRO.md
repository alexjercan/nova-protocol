# Retro: Folder bundle - bundle.ron manifest + base game as a bundle

- TASK: 20260714-134119
- BRANCH: modding/folder-bundle
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Building on the 150508 content-model foundation made this task small: the
  merge-by-kind router already existed, so the folder bundle was "manifest +
  directory-of-content-files + overlay", not per-extension routing. The
  generic-first re-sequencing (spike 150410) paid off exactly as intended - no
  bespoke-then-fold detour.
- Folding base-as-bundle (134123) in as the proof kept the mechanism honest:
  the end-to-end `demo_scenario` test (gating on the recursive dependency load
  state) and both windowed examples exercised the real bundle path, not a
  throwaway demo bundle.
- The out-of-context `/code-review` pass caught the one real hazard (section
  overlay was first-wins/append, unlike the scenario map's last-wins/insert) -
  a latent bug that only bites the NEXT task (mods, 134127). Flagging it now,
  before mods, is the cheap moment.

## What went wrong

- R1.1 (section overlay-by-id): the initial router used `sections.push` while
  scenarios used `insert`, so the two kinds silently diverged - a mod's section
  override would have been appended as a shadowed duplicate that
  `get_section`'s first-match ignores. Root cause: sections are a Vec and
  scenarios a map, so "route by kind" was written per-container without
  noticing the two containers now needed the SAME overlay semantics. Behavior
  was correct for the single base bundle (no id collisions yet), so no test
  failed - the divergence was invisible until a second bundle exists.
- R1.3 (the `to_string` NIT): I applied the reviewer's suggested `rel.as_str()`
  before compiling and it broke (`E0597`: the borrow does not outlive the
  resolved path). The owned `to_string()` was load-bearing, not a smell. Cost
  one compile cycle. Lesson: a "remove this alloc" NIT is a hypothesis - verify
  it compiles before treating it as done.

## What to improve next time

- When one router dispatches into multiple containers that share an id space,
  make them share the OVERLAY helper, not just the match. Extracting
  `merge_content_item` up front would have made the two kinds obviously
  identical and the divergence impossible.
- Treat a reviewer's micro-optimization NIT as unverified until it compiles;
  borrow-vs-own suggestions especially.

## Action items

- [x] Extracted `merge_content_item` + pinned overlay-by-id with two unit tests
      (section in-place replace, scenario insert).
- [x] Documented that the `to_string` in the bundle path resolve is
      load-bearing (comment at the call site).
- [ ] 134127 (mods) can now rely on last-wins overlay for both kinds - no
      further router change needed; it just appends bundle handles.
