# Notes: nova_probe commit-keyed run roots

- TASK: 20260729-003352
- BRANCH: tooling/probe-commit-keyed-runs

## Current behavior found

Before this change, `probe run <example>` wrote directly to
`probe-runs/<example>/` unless `--out <dir>` was supplied, in which case the
exact directory from `--out` was the run directory. Multi-spec runs wrote an
aggregate root at `probe-runs/` or the exact `--out` directory, with each
example under `<root>/<example>/`. Baselines were explicit: a single run used
`--baseline <run-dir>` directly, and a multi run treated `--baseline <root>` as
the root containing `<example>/frametime.csv`. `probe report` accepted one dir
and re-rendered either that run dir or an aggregate root with `probe-all.json`.

## What changed

All native run specs now use one aggregate-shaped driver. A single example is a
one-item vector, so it writes the same root artifacts as a multi run:
`index.html`, `index.json`, and `probe-all.json` at the commit root, plus the
per-example `report.html`, `checks.json`, and `probe-run.json` under
`<root>/<example>/`.

The output path model is now storage-base first. `--out <base>` means "write
under this base", not "write exactly here". The run root is always
`<base>/<short-commit>/`, defaulting to `probe-runs/<short-commit>/`, and each
example lands under `<base>/<short-commit>/<example>/`.

`--baseline <base>` now means "search this base for the nearest previous commit
hash directory". When `--baseline` is omitted, probe searches the same base as
`--out`, defaulting to `probe-runs`. Discovery walks `git rev-list` order,
ignores the current commit, ignores non-hash directories such as `before`, and
uses the closest existing ancestor directory. If git history is unavailable or
no matching directory exists, the FPS comparison is skipped with a clear message.

The manifests now include `full_git_sha` alongside the existing short
`git_sha`, so a short directory can be mapped back to the exact revision that
produced it.

`probe report` now accepts multiple run or aggregate dirs in one invocation and
re-renders each. The task brief called this `nova_probe render`; the actual
current verb is `report`, so the implementation and tests use the live CLI.

## Compatibility

No historical probe artifacts were edited. Explicit legacy baselines are still
accepted: if an explicitly supplied baseline root is an old direct run dir with
`frametime.csv`, or an old root with `<example>/frametime.csv`, probe can use it.
Auto-discovery does not pick compatibility folders, because selecting names like
`before` or `playable` automatically would reintroduce the ad hoc ambiguity this
task removes.

The tradeoff is that `--out` is no longer an exact artifact directory. That is
intentional after the run model was simplified: `--out` names the storage base,
and the commit key is always inserted by probe.

## Bugs and diagnostics

The first test-first compile failed on the intended missing helpers:
`report_many`, `default_output_root`, and `discover_baseline_root`. After wiring
the helpers and aggregate driver, `cargo test -p nova_probe` passed. A later
review of the compatibility behavior found that explicit `--baseline
probe-runs/before` would have stopped working if all explicit baselines were
forced through hash discovery, so the resolver now has an explicit-only
compatibility fallback and a test proving auto-discovery still ignores those
folders.

## Verification

- `nix develop --command cargo test -p nova_probe` passed after the
  implementation.
- Added focused tests for commit-keyed output roots, baseline ancestor
  discovery, explicit compatibility baselines, old/new baseline shapes, parser
  support for multiple report dirs, and actual multi-dir report re-rendering.
- Updated `README.md`, `web/src/wiki/dev/development.md`, CLI help text, and
  source docs/comments that described the old `probe-runs/<example>` layout.

## Self-reflection

The useful correction was treating single-run as a one-item aggregate instead
of preserving a separate single-run path. I initially started with a narrower
patch that kept more branching than needed; next time, when the command already
has an aggregate mode, I should first ask whether the single-item case can be
made the primitive instead of special-casing it.
