# Retro: CI PR checks (task 20260709-140816)

## What changed and why

AGENTS.md told agents to skip local `cargo test`/`cargo clippy` because "both
run in CI on every PR", but the only in-repo workflows were `deploy-page.yaml`
(dispatch) and `release.yaml` (tags). Nothing ran the suite on PRs, so the "CI
is the source of truth" claim was aspirational. This task added
`.github/workflows/ci.yaml` running `cargo fmt --check`, clippy (a plain
`--workspace --all-targets` pass plus a `--features debug` pass) and
`cargo test --workspace` on `pull_request` and pushes to `master`, and pointed
AGENTS.md at the workflow so the claim is now verifiable.

Shape borrowed from `~/personal/bevy-common-systems/.github/workflows/ci.yml`,
per the user's steer.

## Key decision: runtime graphics stack, not just build headers

The one real divergence from the bevy-common-systems template. That repo
deliberately only *compiles* its examples, so it installs build headers
(`libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`) and nothing
else. Nova-protocol's `tests/examples_smoke.rs` actually *launches* six
harnessed examples under `BCS_AUTOPILOT` - they use `DefaultPlugins` and open a
real window - so the CI also needs:

- a Vulkan loader + mesa lavapipe (`libvulkan1 mesa-vulkan-drivers`) to render
  with no GPU on the runner, and
- Xvfb (`xvfb-run --auto-servernum`) to give winit an X display; the test skips
  loudly when `DISPLAY` is unset, so without Xvfb CI would have gone green
  while silently testing nothing.

Added a `timeout-minutes: 30` job cap because the smoke test spawns gameplay
subprocesses with no internal timeout; under software rendering a wedged
example would otherwise hang the runner up to GitHub's 6h limit.

Ran the debug variant for clippy only, not for the full test pass: the smoke
test already exercises the examples with `--features debug` at runtime, and a
second `cargo test --features debug` would re-run the ~1-2 min smoke suite for
little extra coverage.

## Difficulties

- No offline YAML validator or `actionlint` in the environment (no pyyaml, no
  ruby/node, no network). Fell back to a tab check + structural comparison
  against the two proven existing workflows. The file has not been executed on
  a real runner yet, so the lavapipe/Xvfb path is reasoned-correct but
  unverified end to end - first PR against the repo is the real test.
- Local `cargo test`/`clippy` are intentionally skipped per repo memory, so
  verification was limited to `cargo fmt --check` (clean) on the branch.

## What could have gone better / next time

- The biggest residual risk is whether lavapipe actually renders the Bevy
  examples on `ubuntu-latest` within 30 min. If the first CI run fails or times
  out, likely follow-ups: pin the Bevy-recommended headless env (e.g.
  `WGPU_BACKEND`/adapter selection), or add `vulkan-tools` + a `vulkaninfo`
  debug step to confirm lavapipe is the enumerated device.
- Consider a lighter/faster PR job (fmt + clippy + non-smoke tests) separate
  from the heavier windowed smoke job, if the ~1-2 min (plus example compile
  time) proves annoying on every push.
