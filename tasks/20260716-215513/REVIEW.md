# Review: Consolidate demo + variety into ONE self-contained 'example' tutorial mod

- TASK: 20260716-215513
- BRANCH: example-mod

## Round 1

- VERDICT: APPROVE

Reviewed via an independent, out-of-context agent (the implementer and reviewer
shared this session, so a fresh-eyes pass re-derived the load-bearing claims:
RON shapes cross-checked against `crates/nova_scenario/src/{actions,loader}.rs`
and the base scenarios; every filter/action/orbit target id verified spawned;
the win gate confirmed reachable and soft-lock-safe; the installed-mod COUNT
assertions confirmed correctly repointed, not weakened). No BLOCKER/MAJOR/MINOR
findings.

Confirmed PASS on all five axes:
- RON correctness: all shapes/fields valid, all prototype + entity ids resolve,
  `destroyed > 1` gated by `arena_done == 0` is reachable and overshoot-immune,
  both self:// refs name declared resources, `.meta` sidecar present.
- Spec completeness: section overlay, new section, playable scenario +
  objective + win, self:// skybox/.meta/texture, `menu_backdrop` scene, two
  StoryMessage beats, Victory AND Defeat Outcomes - all present.
- Test integrity: counts 3->2 (example_scenario) and 4->3 / 3->2 with the
  downloaded-row index shift (mod_cache_install) correctly repointed; overlay
  assertions (health 400, name contains "Example Mod", example_arena present)
  still fail if the overlay breaks; nothing weakened for green.
- Sweep: no missed LIVE references to the removed mods. Remaining `demo` hits
  are generic in-memory test doubles (invented authors, a fabricated `reel`
  mod) that load nothing removed.
- Honesty/docs: the four wiki guides, the binary-resources design doc and the
  CHANGELOG accurately describe the single example mod.

Bug caught and fixed during self-review before this round (commit 2e03e2de):
the menu backdrop's AI orbiter pointed at a stale well id after the
`menu_planetoid` rename - now points at the spawned well.

NITs (addressed):

- [x] R1.1 (NIT) crates/nova_assets/src/mod_refs.rs:159 - test fixture `name:
  "Variety Demo"` was a stale label after the `id` was renamed to
  `example_scenario`; relabeled to "Example Demo".
- [x] R1.2 (NIT) webmods/gauntlet/gauntlet.content.ron:12 - comment referenced
  "the demo mod's hull override that v1.0.0 rode", a mod that no longer exists;
  reworded to be mod-agnostic ("NOT by any mod's overlay of that section").
- [x] R1.3 (NIT) crates/nova_mod_format/src/lib.rs:224,254,268 - decoder-fixture
  strings ("Demo Mod", `mods/demo/demo.bundle.ron`) coincidentally echo the
  removed mod. LEFT AS-IS (deliberate): these are generic RON-decoder test
  doubles that load nothing - the same fixture also declares a fabricated
  `reel` mod and an `other-mod` dependency, so the strings are arbitrary
  parser input, not references to the shipped mod.
