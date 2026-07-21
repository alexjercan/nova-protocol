# Retro: crate-scoped tests work standalone (self dev-dep feature)

- TASK: 20260721-000249
- BRANCH: fix/crate-solo-tests (landed 1b7b5dff)
- REVIEW ROUNDS: 1 (out-of-context APPROVE, no findings)

See TASK.md close-out for what changed; process only here.

## What went well

- Reproduce-first held: rather than blindly self-dep every serde crate, the impl
  ran `cargo test -p X --no-run` and found only nova_scenario actually fails
  (ungated `ron` round-trip tests), fixing exactly one crate and correctly
  declining to touch nova_gameplay (its serde test IS cfg-gated) and nova_core
  (a false-positive `debug` gate). No needless deps.
- The warm sccache from the prior task made this task's many per-crate test
  builds fast - the build-ergonomics investment paid off immediately on the very
  next task.

## What went wrong

- A latent methodology risk that review caught: the impl's candidate sweep was
  `grep cfg(feature)` - but nova_scenario's OWN failing tests are UNGATED, so
  that grep would have missed nova_scenario itself if it were not on the known
  list. The completeness of "only nova_scenario is affected" rested on a proxy
  (grep) that is blind to exactly the failure mode being fixed.

## What to improve next time

- For a "make X work across ALL crates" task, prove COMPLETENESS by RUNNING the
  real check per crate, not by grepping for a marker - the failing case may lack
  the marker (nova_scenario's failing tests carried no `#[cfg(feature)]`). Review
  did the right thing: `cargo test -p <crate> --no-run` for all seven serde
  crates, proving all-but-one compile solo (and finding the real reason: only
  nova_gameplay + nova_scenario declare serde OPTIONAL; the rest carry it
  non-optionally so were never at risk).

## Action items

- [x] LESSONS.md: added `completeness-by-running-not-grepping` (x1).
- This completes the 20260719-002512 spike's two seeded tasks (sccache +
  crate-solo). The AGENTS.md feature-unification incantation is gone.
