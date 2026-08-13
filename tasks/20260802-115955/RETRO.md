# Retrospective

## What worked

- Naming steps and waiting on observed state turned vague hangs into specific
  failed seams.
- A capability handshake made missing measurements visible instead of green.
- One ordinary script model now serves smoke, capture, correctness, and probe
  evidence.
- Rebuilding screenshot producers gave the website current, reproducible art.
- Strict automation exposed real input, lifecycle, spawn-order, and content
  defects before release.

## What changed during delivery

- Automation ownership moved from BCS to the workspace-local `nova_autopilot`.
- Screenshot packaging stayed in Python; probe owns correctness and profiling.
- Mainline story examples were retired in favor of code-built system fixtures.
- The release expanded beyond tooling when better evidence exposed implicit
  ship structure, lifecycle aliases, and unclear crate ownership.
- Semantic parts and link-point mates became the final visible expression of
  the same rule: authored intent must not be inferred from incidental geometry.

## Next time

- Run the exact clippy gate before the first release-frontier push. The final
  CI found a cheap example-only lint after broader checks had passed.
- Keep performance claims narrower than the evidence. Host-noisy comparisons
  are review warnings, not automatic regressions.
- Add release-post captures to the screenshot manifest when drafting the post,
  not after its first review.
