# Notes: Make standard nova_probe_cli diagnostics automatic

## Problem Statement

Remove `--fps` and `--profile`. A normal native `nova_probe_cli run` enables
both behaviors by default. Do not add negative flags.

Frame-time collection is conditional on the program's runtime probe contract:
run the frame-time pass if and only if the clean run declares `frametime`.
`NovaProbePlugin::default()` wires frame time, timeline, and invariants, so its
users get the normal full probe without more CLI configuration.

Keep `--samply` as the only opt-in diagnostic pass because it is slow.

## Context

- `RunOptions` currently stores independent `fps`, `profile`, and `samply`
  booleans.
- The clean pass arms the contract writer and produces `probe-contract.json`.
- `nova_frametime()` declares `Capability::FrameTime` even when its capture
  environment is not armed. The runner can therefore inspect the clean-pass
  contract before deciding whether to launch the separate frame-time pass.
- `NovaProbePlugin::default()` enables all three runtime capabilities.
- Frame-time must remain a separate pass. The recorder flush on the frame path
  contaminates the measurement if frame time rides the clean pass.
- The traced pass must remain separate because its instrumentation changes
  timings.
- Trace/profile has no runtime `Capability`. It is a CLI build and run mode
  using the `trace` feature. Removing `--profile` therefore means this pass is
  unconditional for normal native runs, not contract-selected.
- The current 2x2 frame-time state is `declared/not declared` x
  `armed/not armed`. Automatic scheduling reduces current runs to:
  `declared -> armed and measured`; `not declared -> N/A`.
- Historical manifests can still contain `armed.fps = false`. Report parsing
  must remain compatible if old run directories are supported.
- Matrix runs currently require `--fps` and measure frames inside each matrix
  cell. Web runs reject `--fps` but always capture their frame line. Removing
  the flag requires those validation branches and help text to be rewritten,
  not only default booleans.
- The capability rule applies to normal, matrix, and web runs without an
  exception: declared frame time -> collect it; undeclared frame time -> do
  not collect it and report N/A. Matrix intent does not override the program's
  runtime contract.

## Questions

- None.

## Ideas

- Normal native flow: clean pass -> parse clean `probe-contract.json` -> if it
  declares `frametime`, run the dedicated frame-time pass -> always run the
  traced pass -> optionally run samply.
- Matrix flow follows the same declaration rule. Do not retain the current
  `--scenario`/`--preset` implies `--fps` validation as a new capability error.
  Arming a matrix cell can be inert when the frame-time plugin is absent; the
  report resolves that case from the runtime contract as N/A.
- Remove `fps` and `profile` from `RunOptions`, parsing, usage, combination
  gates, pass counts, and examples. Keep only facts in `RunManifest`: which
  passes actually ran and which surfaces were armed.
- Do not model the removed operator choice in new reports. Contract absence is
  N/A. A declared capability that was scheduled but produced no artifact is a
  failure.
- Keep old manifest parsing tolerant. New output can retain an execution fact
  such as `armed.fps`, or derive it from the recorded `fps` pass. Decide during
  implementation based on the smallest compatible format change.
- Update the task wording that says examples without frame-time wiring do not
  get a frame-time run. This is achievable after clean-pass contract discovery.
