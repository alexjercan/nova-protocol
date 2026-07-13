# CI: examples smoke test panics in taffy on GitHub runners only - diagnose and re-enable as blocking

- STATUS: CLOSED
- PRIORITY: 20
- TAGS: v0.5.2, ci, testing, bug

The `examples_smoke` test step in `.github/workflows/ci.yaml` is currently
`continue-on-error: true` because the scenario-loading example (03_scenario
then; rebuilt as `08_scenario` in 20260712-211352) deterministically panics on
GitHub's ubuntu-latest runners with:

```
thread 'Compute Task Pool (N)' panicked at taffy-0.10.1/src/util/resolve.rs:68:18:
internal error: entered unreachable code
Encountered a panic in system `bevy_ui::layout::ui_layout_system`!
```

Full background in NOTES.md in this task folder (section
"The taffy panic"). Summary of what is already ruled out:

- NOT the skybox cubemap upload race (fixed in v0.4.0; the panic still
  reproduced on CI with the fix in).
- NOT inf/NaN layout values: taffy 0.10.1 computes layout fine with
  inf/NaN sizes and insets (verified with a scratch taffy tree). Hitting
  that `unreachable!()` requires a corrupt style tag, which safe code
  should not be able to produce.
- Does NOT reproduce locally in 15+ runs across: NixOS host (Xvfb +
  lavapipe), Ubuntu 24.04 container with the exact CI Mesa/LLVM stack
  (Mesa 25.2.8, LLVM 20.1.2), and that container limited to 2 CPUs / 15 GB.

Remaining suspect: the runner hardware. llvmpipe JIT-compiles for the host
CPU (AMD EPYC 7763 / zen3 on CI vs Intel locally); a CPU-specific JIT bug
corrupting heap memory in-process would explain garbage tags in an
unrelated safe-Rust system.

Steps:

- [~] Collect the full backtrace from CI: SUPERSEDED - the panic did not
      survive the 20260712-211352 examples rework (see Record), so there
      is no backtrace to collect.
- [~] LP_NATIVE_VECTOR_WIDTH / SwiftShader / Mesa experiments: SUPERSEDED,
      same reason - no reproduction left to experiment against.
- [~] qemu-user `-cpu EPYC` reproduction: SUPERSEDED, same reason.
- [x] Remove `continue-on-error: true` from the smoke step so it gates
      again - done, over the reworked 12-example suite.

## Notes (v0.5.2 plan pass, 2026-07-13)

- Depends on: 20260712-211352 (examples rework) - re-enable the gate over
  the final example set, not the pre-rework one.
- The CI experiments (backtrace, LP_NATIVE_VECTOR_WIDTH, SwiftShader/Mesa
  swaps) only run on GitHub's runners, so this task needs branches pushed
  to origin. Pushing is the user's call: prepare the experiment commits,
  then stop and ask before the first push. The qemu-user `-cpu EPYC`
  reproduction attempt is the one experiment that runs locally.
- If the panic stays unexplained after the experiment budget, the fallback
  is an explicit containment: keep the panicking example out of the blocking
  set with
  a written justification, gate the rest, and leave this task's NOTES.md as
  the investigation record - do not leave the whole suite non-blocking.


## Record (2026-07-14)

Resolution: worked around by replacement rather than root-caused. The
panic was observed deterministically on the pre-rework 03_scenario; the
20260712-211352 rework rebuilt the entire suite (03_scenario's content
lives on as 08_scenario, a different scene and script), and the first
master push after it ran the FULL reworked suite green on ubuntu-latest -
including the new in-example behavior assertions and the command-error
gate (run 29283727248; the STEP conclusion is maskable under
continue-on-error, so the evidence is the job log itself: `test result:
ok. 1 passed; 0 failed ... finished in 188.23s` for
tests/examples_smoke.rs, whose single test iterates all twelve
HARNESSED_EXAMPLES - review R1.2). A deterministic failure that no longer fires after its trigger
was replaced is resolved for gating purposes; the gate is blocking again.

What we never learned: the actual corruption mechanism (the llvmpipe
zen3-JIT theory was never confirmed or refuted - the local
non-reproduction evidence and the empirical taffy-tag analysis remain in
NOTES.md). If the panic resurfaces, the now-blocking step fails loudly
with RUST_BACKTRACE=full and this task's NOTES.md is the starting point;
the planned experiments (LP_NATIVE_VECTOR_WIDTH, SwiftShader, qemu EPYC)
are still the right ladder.

Self-reflection: sequencing this task AFTER the examples rework (a plan
call) converted a hardware-forensics investigation into a one-line gate
flip - the cheapest possible resolution. The discipline point: the close
is honest that this is containment-by-replacement, not understanding.
