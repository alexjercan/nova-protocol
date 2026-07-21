# Review: fix crate-scoped tests via self dev-dep feature

- TASK: 20260721-000249
- BRANCH: fix/crate-solo-tests

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings from a fresh reviewer with no sight of the implementing
session. It verified the reproduce-first (on master `cargo test -p nova_scenario
--no-run` FAILS with `ScenarioConfig: serde::Serialize is not satisfied`; on the
branch it passes 131), the fix shape (self dev-dep, NOT `required-features` -
that word appears only in an explanatory comment), and the workspace
(`cargo check --workspace --all-targets` clean bar the pre-existing
proc-macro-error2 warning).

The crux was COMPLETENESS - the implementer's `grep cfg(feature)` sweep is the
same method that would have missed nova_scenario's own UNGATED failing tests, so
another serde crate could silently fail standalone. The reviewer INDEPENDENTLY
ran `cargo test -p <crate> --no-run` for ALL seven serde crates (nova_assets,
nova_gameplay, nova_events, nova_menu, nova_modding, nova_mod_format,
nova_scenario): all seven compile solo; only nova_scenario ever failed. Root
insight: only nova_gameplay and nova_scenario declare serde as an OPTIONAL
feature; the other five carry serde non-optionally, so they were never at risk.
The implementer's scoping (fix only nova_scenario) is therefore PROVEN complete,
not just asserted - the DoD ("no crate needs a feature incantation") is met.

- No BLOCKER/MAJOR/MINOR/NIT. Docs correct (AGENTS incantation dropped, LESSONS
  marked fixed-at-root, modding-ron.md's `cargo build` note correctly left as an
  accurate architecture statement). Reproduce-first honest, scoping complete.
