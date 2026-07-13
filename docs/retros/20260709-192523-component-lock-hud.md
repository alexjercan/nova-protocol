# Retro: Component-lock HUD (markers, highlight, focus meter)

- TASK: 20260709-192523
- BRANCH: feature/component-lock-hud (squash-merged as 58fc6c7)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Closes the component-lock arc. What shipped is in the task's Resolution and
docs/retros/20260709-component-lock.md.

## What went well

- **Every pattern was a reuse.** Reconcile membership (turret pips), Entity
  anchors (widget), guarded style writes (substrate R1.1 lesson),
  discriminating example probes (pin the TAIL section so snap cannot
  produce the same result). The whole HUD task produced zero review
  findings because its shapes were all previously reviewed shapes.
- **The example caught real environment truths, not code bugs.** All four
  new stages passed on the first complete run; the two failed runs before
  it were tooling (cargo rebuild loop, asset root) - the assertions
  themselves were right the first time.

## What went wrong

- **`cargo run` rebuilt the entire dependency graph right after a
  successful `cargo build` of the same target**, twice eating the run
  timeout. Running the built example binary directly sidesteps it, but
  needs `BEVY_ASSET_ROOT=$PWD` (bevy resolves assets against the executable
  outside cargo). Root cause unidentified (fingerprint instability in this
  environment); the workaround is now the range runbook: build once, run
  the binary with BEVY_ASSET_ROOT.
- **A range binary run without the asset root fails as a vacuous timeout**,
  not a clear error: loading never finishes, Playing never arrives, and
  only the asserted-at-exit backstop (thankfully) fired. The backstop
  lesson from the com-range retro is what turned a silent hang into a
  diagnosable panic - compounding in action.

## What to improve next time

- Range runbook: `cargo build --example X --features debug` once, then
  `BEVY_ASSET_ROOT=$PWD DISPLAY=:99 BCS_AUTOPILOT=1 ./target/debug/examples/X`.
  Treat a repeated full-dep recompile under `cargo run` as environment, not
  code.

## Action items

- None new. Arc complete; deferred items are recorded in the arc doc
  (radial ring, AI component picks, faction hostility, audio cues).
