# CI: examples smoke test panics in taffy on GitHub runners only - diagnose and re-enable as blocking

- STATUS: OPEN
- PRIORITY: 10
- TAGS: ci,testing,bug

The `examples_smoke` test step in `.github/workflows/ci.yaml` is currently
`continue-on-error: true` because `03_scenario` deterministically panics on
GitHub's ubuntu-latest runners with:

```
thread 'Compute Task Pool (N)' panicked at taffy-0.10.1/src/util/resolve.rs:68:18:
internal error: entered unreachable code
Encountered a panic in system `bevy_ui::layout::ui_layout_system`!
```

Full background in docs/2026-07-10-skybox-cubemap-upload-race.md (section
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

- [ ] Collect the full backtrace from CI (the non-blocking step now runs
      with RUST_BACKTRACE=full) and confirm the panic site and thread.
- [ ] Try steering the llvmpipe JIT on CI: `LP_NATIVE_VECTOR_WIDTH=128`
      env for the smoke step is the cheapest experiment (CI logs report
      "llvmpipe (LLVM 20.1.2, 256 bits)").
- [ ] If that does not help, try a different software Vulkan driver on CI
      (SwiftShader) or a newer Mesa (kisak PPA) to rule the driver in/out.
- [ ] Consider reproducing under qemu-user with `-cpu EPYC` to test the
      zen3-JIT theory off-CI.
- [ ] When the panic is understood or worked around, remove
      `continue-on-error: true` from the smoke step so it gates again.
