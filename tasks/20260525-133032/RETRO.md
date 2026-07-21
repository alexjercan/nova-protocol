# Retro: breadth rustdoc pass + missing_docs lint

- TASK: 20260525-133032
- BRANCH: docs/breadth-rustdoc (landed c583a425)
- REVIEW ROUNDS: 1 (out-of-context APPROVE, no findings)

Process only; see TASK.md checklist for the per-crate before/after.

## What went well

- Scoped a 557-item mechanical sweep HONESTLY instead of grinding it: the DoD's
  valuable core (every public plugin/component/resource/event TYPE documented,
  everywhere) + locking the small/medium crates with `#![warn(missing_docs)]`,
  and the large-crate non-category tail (233 + 144 internal fns/config-structs)
  filed as a follow-up (20260721-121316) rather than force-fit into one pass.
- The measure-first step (`--force-warn missing_docs` workspace build -> per-crate
  counts) turned "large and vague" into a concrete checklist and a clean
  split (which crates can go fully clean vs category-only).
- The critical footgun - enabling `#![warn(missing_docs)]` on a crate that is
  still dirty (it then warns / trips a `-D warnings` CI gate) - was verified per
  crate by both the impl and the reviewer: all 11 enabled crates emit 0
  missing_docs. Bullet 1 (0 undocumented category types) was re-verified by the
  reviewer inspecting every remaining undocumented item in the two large crates
  and confirming none is a category type.

## What went wrong

- Nothing shipped wrong. The one honesty note: the impl fanned the large-crate
  sweep to nested parallel subagents; that is fine BECAUSE it re-verified their
  output with an audit script + a clean build rather than trusting them.

## What to improve next time

- When enabling a "warn-as-clean" lint per crate, the acceptance check is
  per-crate ZERO-of-that-warning (not just "cargo doc passed") - a lint enabled
  on a dirty crate is a silent CI liability. Measure each enabled crate.

## Action items

- [x] LESSONS.md: added `lint-enabled-crate-must-be-zero-of-that-warning` (x1).
- Follow-up 20260721-121316 (nova_scenario + nova_gameplay non-category tail +
  their lint) is now queued on master.
