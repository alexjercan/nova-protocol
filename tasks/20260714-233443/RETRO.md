# Retro: WebGPU-detection gate at the Play boundary

- TASK: 20260714-233443
- BRANCH: feat/webgpu-detection-gate
- REVIEW ROUNDS: 1 (APPROVE, one NIT)

## What went well

- The plan's verify-first step ("confirm whether trunk's auto-init can be cleanly
  prevented") resolved cleanly into a concrete mechanism: trunk emits its wasm
  bootstrap as a deferred `<script type="module">`, so a plain inlined `<script>`
  placed after `.game-container` runs synchronously first. Verified it in the built
  `dist/index.html` (gate at ~line 170, trunk module at ~238) rather than trusting
  the reasoning - the whole no-black-flash design rests on that ordering.
- Tested the ACTUAL shipped `webgpu-check.js` by running it in a node `vm` with a
  stubbed DOM (no new deps), so the test covers the file trunk inlines, not a copy.
- Folded the mid-cycle playtest into this task instead of deferring it: the user's
  Firefox/Linux crash both proved the task's premise and exposed a real gap, and it
  became a regression pin (the "present but no adapter" test case).

## What went wrong

- The first cut detected WebGPU by `navigator.gpu` presence only. The playtest
  showed that is insufficient: the crash is at *surface/adapter creation*, so a
  browser can expose `navigator.gpu` yet still fail to get an adapter, and
  presence-only would sail past it into the same crash. Root cause: I took the
  plan's assertion at face value - it literally said "requesting an adapter is
  async and unnecessary to gate" - and detected the API namespace instead of the
  capability whose absence causes the failure. The failure mode lives one step
  downstream of the thing I was checking for.
- It took a real browser to catch this; the unit tests I wrote first were faithful
  to the wrong spec, so they were green and useless against the actual failure.

## What to improve next time

- When writing a feature-detection gate, detect by *acquiring the resource whose
  absence causes the failure* (here `requestAdapter()`), not by checking that the
  API object exists - especially when the error you are preventing happens at
  acquisition time.
- Treat a plan assumption explicitly marked "unnecessary" as a claim to re-check
  when it is load-bearing for correctness, not a settled decision.

## Action items

- [x] Added `capability-detect-by-acquiring` to the ledger.
- [x] Added domain lesson `trunk-inline-script-before-deferred-module`.
- Live in-browser eyeball (message on Firefox/Linux, particles on Chrome) via
  `scripts/preview-web.sh` remains a manual step for the user - noted in both task
  NOTES.md, not a new task (the gate logic is unit-pinned and wiring is verified).
