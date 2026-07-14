# Review: Add navigator.gpu WebGPU-detection gate at the Play boundary

- TASK: 20260714-233443
- BRANCH: feat/webgpu-detection-gate

## Round 1

- VERDICT: APPROVE

Independently re-verified the load-bearing claims rather than trusting the summary:
- Ran `build/web/webgpu-check.test.mjs` myself - 5/5 pass, and they exercise the
  ACTUAL shipped file in a vm (not a copy). The "present but no adapter" case is
  failable: delete the async `requestAdapter()` probe and it goes red, so it
  genuinely pins the Firefox/Linux crash the playtest exposed.
- Re-grepped the built `dist/index.html`: `.game-container` (161), the gate as a
  plain `<script>` (170), trunk's `<script type="module">` (238). A plain script is
  synchronous and a module is deferred, so the gate provably runs before bevy boots
  - the whole no-black-flash design rests on this and it holds in the real artifact.
- Confirmed the `../` back link (resolves `/nova-protocol/play/` ->
  `/nova-protocol/` at the deploy subpath).

Correctness of the edge cases is sound: `navigator.gpu` absent -> sync fallback;
present + null/rejecting adapter -> async fallback; `requestAdapter` missing/throwing
-> caught by the try/catch -> fallback. The async probe cannot cause a false
positive on a working browser (requestAdapter returns an adapter there), and calling
it alongside bevy's own request is harmless (adapters are not exclusive). The
plan-deviation (landing CTA stays clickable with a note rather than hard-disabled)
is justified and documented. Tests/docs/CHANGELOG all present. Good, careful work
that got better under a real playtest.

- [x] R1.1 (NIT) web/src/webgpu.ts:15 - the landing warning uses presence-only
  (`"gpu" in navigator`), so it will NOT warn on a browser that exposes the API but
  cannot get an adapter (Firefox/Linux with the pref flipped) - whereas the game
  page DOES catch that via the adapter probe. Acceptable asymmetry (the landing note
  is courtesy; the game page is the authoritative gate), but either mirror the
  async `requestAdapter()` check here too, or add one line to NOTES stating the
  landing layer is presence-only by design. No behavior risk.
  - Response: Documented - added a line to NOTES.md ("Two layers", layer 2) stating
    the landing layer is presence-only by design and the game page is the
    authoritative catch for present-but-broken WebGPU. Kept the landing check
    synchronous/simple deliberately rather than mirroring the async probe.
