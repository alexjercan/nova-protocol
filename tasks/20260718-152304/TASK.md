# Tooling inventory + consolidation pass: catalog every bin/script, decide what merges or moves to build-time, document the result

- STATUS: CLOSED
- PRIORITY: 26
- TAGS: v0.8.0, tooling, refactor, docs

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

- [x] Catalog every dev entry point with its purpose, invocation, and
      dependencies:
  - [x] Rust bins: `content` (lint only after the pre-made moves below;
        was gen/lint/audit), `nova_probe` (perf_web + the report bin once
        20260718-152230 lands), `nova_meta_gen`, `nova_portal_gen`.
  - [x] scripts/: preview-web.sh (perf-* scripts retired into probe),
        gen-licenses.sh, gen-web-screenshots.py, gen-placeholder-sounds.py,
        cut-obj-into-hulls.py.
- [x] For each, classify: keep-as-Rust, port-to-Python (portal
      20260718-152247, maybe meta 20260718-152255), fold-into-build-step, or
      leave. Note what is a true build-time hook (meta gen on Trunk build;
      content gen as a pre-commit / CI gate; portal gen on deploy) vs an
      on-demand dev tool.
- [x] Recommend the consolidated structure (e.g. a single `scripts/` home + a
      documented build-time hook list; whether a task-runner/Justfile is worth
      it) and record it so later tasks execute against one target picture.
- [x] Sequence the concrete tooling tasks against the map (which move first,
      which wait) and note it in each task if the order matters.
- [x] Update the README tools section (with 20260718-152205) + wiki
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
    cubemaps), not just local `trunk serve`. Python-vs-Rust is RESOLVED (spike
    20260718-152255, 2026-07-20): STAYS RUST - it asks Bevy for each loader's
    default meta (Bevy-version-specific loader paths + settings fields), so a
    Python hardcode would silently drift and break web mod cubemaps. Portal gen
    ported to Python (152247), meta gen does not. LOCATION resolved too (spike
    152255 round 2 -> task 20260720-224236): it moved OUT of `crates/` into
    `tools/nova_meta_gen`, staying a workspace member (feature unification pins
    its Bevy to the game's) but excluded from `default-members` so bare builds
    skip the web-only tool.
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

## Closeout (2026-07-20): classification table + Justfile decision

The inventory (every entry point, invocation, purpose) is delivered by the
README "Tools" section (commit a0e3393d) + `web/src/wiki/dev/development.md`.
This closeout records the two pieces unique to this umbrella: the keep/port/
build-time CLASSIFICATION with each tool's home, and the Justfile yes/no.

### Classification table

| Entry point | Lang | Home | Classification | When it runs |
| --- | --- | --- | --- | --- |
| `content` (gen + lint) | Rust | `crates/nova_assets/src/bin` | keep-Rust | on-demand author + CI gates (`content_lint_gate`, `balance_audit_gate`) |
| `nova_probe` (run / report) | Rust | `crates/nova_probe` | keep-Rust | on-demand run-harness (post-feature verify) |
| `perf_web` | Rust->wasm | `crates/nova_probe/src/bin` | keep-Rust | driven by `probe run --platform web` |
| `scenario_dispatch` bench | Rust | `crates/nova_scenario/benches` | keep-Rust | on-demand `cargo bench -p nova_scenario` |
| `nova_meta_gen` | Rust | **`tools/`** | keep-Rust, RELOCATED | build-time hook (Trunk `post_build`), web only |
| `gen-portal.py` | Python | `scripts/` | PORTED (was `nova_portal_gen`, crate REMOVED) | build-time: deploy workflow + `preview-web.sh` |
| `preview-web.sh` | shell | `scripts/` | keep | on-demand local combined game+site preview |
| `gen-licenses.sh` | shell | `scripts/` | keep | on-demand / release (license aggregation) |
| `gen-web-screenshots.py` | Python | `scripts/` | keep | on-demand (wiki screenshots) |
| `gen-placeholder-sounds.py` | Python | `scripts/` | keep | on-demand (placeholder audio) |
| `cut-obj-into-hulls.py` | Python | `scripts/` | keep (cut-only; classification is in-game) | on-demand (obj -> hull `.glb`) |
| `check-docs-clean.sh` / `wipe-docs.sh` | shell | `scripts/` | keep | release-time (ephemeral-docs guard + wipe) |

### Decisions applied this cycle (the moves the umbrella sequenced)

- `content` `audit` -> merged into `lint` (20260718-152240, landed).
- `content` `gen` -> build-time: DECLINED, stays a subcommand (20260719-092952
  wontdo - a build.rs generator duplicate-compiles bevy + mutates tracked files).
- `nova_portal_gen` -> ported to `scripts/gen-portal.py` (20260718-152247) and
  the Rust crate REMOVED (20260720-230924), gate coverage re-homed onto the
  Python tool.
- `nova_meta_gen` -> stays Rust (asks Bevy, would drift; spike 20260718-152255)
  but RELOCATED out of `crates/` into `tools/` (20260720-224236), a workspace
  member excluded from `default-members`.

### True build-time hooks (vs on-demand dev tools)

- `nova_meta_gen` - Trunk `post_build` (web build only).
- `gen-portal.py` - the deploy workflow + `preview-web.sh`.
- `check-docs-clean.sh` - release/CI guard (ephemeral-docs model).
- CI gates (enforced, not hooks): `content lint` walks (`content_lint_gate`,
  `balance_audit_gate`), `content_ron_parity` (the `content gen` drift gate).

### Justfile / task-runner: NO

Every invocation is already documented in three places (README "Tools",
`development.md`, the AGENTS.md command cheatsheet) and the set is small and
stable. A Justfile would add a `just` toolchain dependency AND a fourth surface
to keep in sync with those docs - and doc-sync drift already reached x7 this
release (`keep-docs-in-sync-with-code`), so a new drift surface is a real cost.
The marginal win (short aliases) does not justify it at this repo size. Revisit
if the entry-point count grows substantially or contributors report
"how do I run X" friction.

Consolidated structure: game code in `crates/`, build/dev tooling split by
language - Rust tools that must link the engine live in `crates/` (content,
probe) or `tools/` (meta-gen, engine-linked but web-build-only); engine-FREE
generators are Python/shell in `scripts/`. That split is the target picture; it
is now realized.
