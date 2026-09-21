# Review 25: Final code edges

- Baseline: `b7a56f0586`
- Lanes: craft, correctness, contracts
- Verdict: major and minor findings

## Findings

### MAJOR - `CHANGELOG.md:12-14` - The changelog banner points to rules that are not in AGENTS.md

The banner says to read the "Changelog section of AGENTS.md" and claims it is the only owner of the release baseline, 200-character cap, and omission rules. No such section exists. AGENTS.md contains only part of the policy; the full cap and intra-cycle omission rules live in `nova-docs/SKILL.md`.

A contributor following the required pointer cannot discover all mandatory changelog rules. Runtime and builds are unaffected.

### MINOR - `tests/env_contract.rs:132-146` - Nine probe environment names are not pinned to owning constants

The roster lists nine `NOVA_PROBE_*` strings that are produced from private string literals in frametime, census, and frame-cost modules. The test compares named constants for other variables but cannot catch a rename of these nine; its source scan will find the stale roster literal itself and stay green.

Current names agree. This is a verification gap for a stable harness interface, not a live mismatch.

### MINOR - `tools/nova_bench/pi/index.ts` is outside TypeScript CI

No tsconfig, lint, format, or workflow includes the PI relay. Rust tests only search its text for tool names. Syntax and type errors are therefore found only when a person runs the PI bench agent.

## Adjudication

Dropped the relay's missing socket timeout as speculative: current referee paths synchronously return a response, so no reachable silent-open state was established.

The previously unassigned `.pi/subagents.yaml` was fully reviewed and found consistent with the review/proof skills and tool permissions.

## Coverage

Fully read the PI relay, all `nova_meta_gen` source/tests, root source/tests/build script, `.pi/subagents.yaml`, remaining build web/CSS/metadata, macOS and Windows build text, hooks, wasm lint config, and root project configuration/document headers. Binary assets were excluded.

Not checked: no Cargo, TypeScript, installer, workflow, build, game, workspace test, or Clippy run. PI config fields were checked against repository conventions, not against an external schema.
