# release-flow: build-macOS fails - x86_64-apple-darwin std missing (E0463 can't find crate for core)

- STATUS: OPEN
- PRIORITY: 5
- TAGS: ci,release,bug

The v0.4.0 release (run 29088287339) uploaded linux, web and windows
assets, but `build-macOS` failed after ~60 min in the "Build release for
x86 Apple" step:

```
error[E0463]: can't find crate for `core`
error[E0463]: can't find crate for `std`
error: could not compile `bytemuck` / `serde_core` / `libc` ...
```

E0463 for core/std when cross-building `x86_64-apple-darwin` (the
universal binary's second half, built on an arm64 macos-latest runner)
means the std library for that target is not installed for the pinned
nightly toolchain. Likely fix in `.github/workflows/release.yaml`: after
installing the toolchain, add

```
rustup target add x86_64-apple-darwin aarch64-apple-darwin
```

(or `rustup toolchain install nightly --target x86_64-apple-darwin`) so
the cross target's std ships with the pinned nightly from
rust-toolchain.toml. The aarch64 half apparently built (the failure is
only in the x86 step), which matches the runner's native arch having std
by default.

Not release-blocking per 2026-07-10 decision: macOS/Windows artifacts are
nice-to-have right now. Fix alongside the next release (v0.4.1) or
whenever the workflow is next touched, and verify with a
`workflow_dispatch` run (version input form `v1.2.3`) instead of a new
tag.
