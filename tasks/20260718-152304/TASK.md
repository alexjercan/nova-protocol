# Tooling inventory + consolidation pass: catalog every bin/script, decide what merges or moves to build-time, document the result

- STATUS: OPEN
- PRIORITY: 26
- TAGS: v0.8.0,tooling,refactor,docs

## Story

As the project owner, I want one coherent map of every dev tool - what it is,
how it runs, and where it should live - so that the individual refactors
(portal port, meta spike, perf report) execute against one target picture
instead of moving pieces ad hoc.

The user wants the tooling refactored into a better structure: "what can be
merged or moved into some kind of buildtime script we do that." Before moving
pieces individually, take one inventory pass so the moves are coherent rather
than ad hoc. Produces the map that the README tools section (20260718-152205)
and the individual refactor tasks consume.

## Steps

- [ ] Catalog every dev entry point with its purpose, invocation, and
      dependencies:
  - [ ] Rust bins: `content` (lint only after the pre-made moves below;
        was gen/lint/audit), `nova_probe` (perf_web + the report bin once
        20260718-152230 lands), `nova_meta_gen`, `nova_portal_gen`.
  - [ ] scripts/: preview-web.sh (perf-* scripts retired into probe),
        gen-licenses.sh, gen-web-screenshots.py, gen-placeholder-sounds.py,
        cut-obj-into-hulls.py.
- [ ] For each, classify: keep-as-Rust, port-to-Python (portal
      20260718-152247, maybe meta 20260718-152255), fold-into-build-step, or
      leave. Note what is a true build-time hook (meta gen on Trunk build;
      content gen as a pre-commit / CI gate; portal gen on deploy) vs an
      on-demand dev tool.
- [ ] Recommend the consolidated structure (e.g. a single `scripts/` home + a
      documented build-time hook list; whether a task-runner/Justfile is worth
      it) and record it so later tasks execute against one target picture.
- [ ] Sequence the concrete tooling tasks against the map (which move first,
      which wait) and note it in each task if the order matters.
- [ ] Update the README tools section (with 20260718-152205) + wiki
      development.md with the final map.

## Definition of Done

- A written inventory exists (in this task or the dev wiki) covering every bin
  and script, each with a classification and a home.
- The Justfile/task-runner question has an explicit yes/no with reasoning.
- README and development.md agree with the inventory, and the concrete
  refactor tasks reference it.

## Notes

- This is the umbrella that sequences the concrete tooling tasks; keep it
  light, it is a plan + doc, not a big code change. Content linter stays
  embedded in `nova_scenario::lint` (good as-is), per the survey.
- Pre-made decisions (user, 2026-07-19) this pass should record and align the
  map to, not re-open:
  - `content` bin `gen` -> build-time: REVERSED (user, 2026-07-20, task
    20260719-092952 CLOSED wontdo). The build-time move was declined - a
    build.rs generator duplicate-compiles bevy and mutates tracked files, and
    routing base-mod gen through Trunk was rejected. `content gen` STAYS as a
    subcommand and `content_ron_parity` remains the drift gate. The bin is
    `gen` + `lint`, not a single `lint`; do not re-open the move.
  - `content` bin `audit` -> merged into `lint`. Balance is a kind of lint, so
    one `lint` command reports reference + balance findings; the `audit`
    subcommand is removed. Own task: 20260718-152240. Net: the `content` bin
    ends up as a single `lint` command.
  - `nova_meta_gen` is ALREADY a build-time hook (Trunk `post_build`), and it
    is needed for the DEPLOYED web build (`AssetMetaCheck::Always`, mod
    cubemaps), not just local `trunk serve`. The open question is only Python
    vs Rust, owned by spike 20260718-152255 - do not double-file it here.
- Do this EARLY in the tooling strand - its whole value is sequencing the
  other tasks; done last it is just a writeup.

## Grooming (2026-07-20): SHRUNK + reprioritized 30 -> 26

The inventory half is already delivered: the README Tools section (commit
a0e3393d) catalogs every bin (content, probe, perf_web, meta_gen, portal_gen,
the dispatch bench) and every script with exact invocation + purpose, and the
probe consolidation (sweep/web/profile/trace retired; scripts/perf_*.sh gone)
already folded the perf scripts. What remains unique to THIS task: (1) the
explicit keep-as-Rust / port-to-Python / fold-to-build-time CLASSIFICATION
table (the README lists tools but not their target home), and (2) the
Justfile/task-runner yes-or-no with reasoning. That is a short writeup, not
an umbrella - the concrete refactors (152247, 152255, 092952, 152240) are
self-contained and do not block on it. Demoted accordingly; close it by
recording those two decisions.
