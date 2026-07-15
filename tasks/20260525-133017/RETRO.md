# Retro: SetSkybox action (swap the scenario cubemap mid-scenario)

- TASK: 20260525-133017
- BRANCH: skybox-action
- REVIEW ROUNDS: 1 (APPROVE)

Process notes only; the what/why is in TASK.md and the code.

## What went well

- **Read bcs's source before designing.** `setup_skybox_camera` does
  `images.get_mut(&cubemap).unwrap()` on `On<Insert, SkyboxConfig>` - it panics on
  an unloaded image. Discovering that by reading the observer (not by shipping a
  synchronous insert and hitting the panic on a modder's first swap) is what made
  the deferred `PendingSkyboxSwap` design obvious. The "small polish" task was
  really an async-asset task in disguise.
- **Reuse paid off twice.** The action copied `SetCamera`'s
  `push_command -> commands.queue(world closure) -> query ScenarioCameraMarker`
  pattern verbatim, and the cubemap authored through the existing `AssetRef` layer
  for free - so the modding surface came out consistent with the format work.
- **Readiness check aligned to the real precondition.** Gating on
  `images.contains(&handle)` (what bcs actually reads) rather than the asset
  server's load state is both correct and testable with a manually-added asset.

## What went wrong

- **A test landed in the wrong module.** I inserted the serde round-trip test by
  anchoring on `object.action(world, info)`, a string that also appears inside the
  `ScatterObjectsConfig::action` *production* impl - so the `#[test]` fn compiled
  inside a trait impl (`the #[test] attribute may only be used on a free
  function`). The compiler caught it instantly, but it was a wasted edit+build.
  Root cause: I matched unique text without confirming which enclosing scope
  (tests module vs a production impl) it belonged to, in a 1800-line file where
  the tests module is not at the end.

## What to improve next time

- When inserting a test into a large file, anchor on an in-module landmark - the
  closing lines of a neighboring `#[test]` - or confirm the module boundary first
  (`grep -n "mod tests"` + its closing `^}`), rather than on any string that
  happens to be unique file-wide.

## Action items

- [x] Follow-up filed: tatr 20260715-140049 - e2e proof of the swap through bcs's
  SkyboxPlugin with a real cubemap (review R1.1).
- [x] Doc note that the applier drops only on a server-reported load failure (R1.2).
