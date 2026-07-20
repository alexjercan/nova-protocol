# Review: Screenshot showcase pipeline (photo-mode actions + reel + web packaging)

- TASK: 20260714-210131
- BRANCH: task/screenshot-showcase-close
- SUBJECT: the task's work landed as commit 92aaf8da; this review critiques that
  commit's diff against its parent (`git diff 92aaf8da~1..92aaf8da`). Scope was
  set with the user to "review + close" - the core goal (Phases 1-4: 22/26
  screenshots + 5 icons, packaged and wired live) is already delivered; the 4
  deferred shots are follow-up task 20260715-004216, and Phase 5 (attract mode)
  is an explicit stretch, both out of scope here.

## Round 1

- VERDICT: APPROVE

Checks run (per repo convention: check/fmt + newly-written tests; the full
clippy/test suite runs in CI, and this work already merged green on master):
- `cargo fmt --check`: clean.
- `cargo test -p nova_scenario --lib actions::`: 16 passed (the new SetCamera /
  Screenshot / resolve_capture_path unit + RON round-trip tests).
- `cargo test -p nova_debug --lib`: 4 passed (reel_pose_camera + reel_capture_path).
- `web`: `npx tsc --noEmit` clean.
- The six examples (13-18) are wired into `tests/examples_smoke.rs`
  `HARNESSED_EXAMPLES`; CI exercises them headless under `BCS_AUTOPILOT`.

Independent re-derivation (implementer and reviewer share a session, so a
load-bearing claim was re-verified rather than read): the PNG IHDR byte offsets
in `png_dimensions` (8-byte signature + 4-byte length + 4-byte "IHDR" tag = 16,
then width/height at 16..24, big-endian `>II`) are correct; `write_png`'s output
was executed standalone and parses back to the expected 44x44.

Design assessment: the engine surface is sound. The load-bearing trick -
`SetCamera` pins a `ScriptedCameraPose` component and the loader enforces it in
`PostUpdate` *after* `WASDCameraSystems::Sync` - is the right fix: a one-shot
Transform set (or merely removing the controller, whose private state survives)
would be overwritten by the free-fly state machine every frame. The reel driver
(`reel_drive`) is a correct pose -> settle -> capture -> await-PNG -> advance
state machine with no out-of-bounds on the final beat (the `index++` lives in the
capturing branch and `done` short-circuits the next frame before re-indexing).
The tests are meaningful (each would fail with its fix removed: the pose test
asserts the component lands and WASD is dropped; the no-camera tests assert
warn-and-continue, not panic).

No BLOCKER or MAJOR findings. The MINOR/NIT items below were cheap and
low-risk, so they were fixed on this branch while it was open rather than
deferred (the branch squash-merges back into the already-landed work):

- [x] R1.1 (MINOR) scripts/gen-web-screenshots.py:112 `png_dimensions` - a file
  that starts with a valid 8-byte PNG signature but is shorter than 24 bytes made
  `struct.unpack(">II", header[16:24])` raise an uncaught `struct.error`,
  aborting the whole run instead of reporting one bad shot. The spec wants every
  bad/mis-sized shot collected and reported together ("fail loudly"), not a
  first-failure crash.
  - Response: FIXED. `png_dimensions` now raises a `ValueError` when the header is
    truncated or lacks the IHDR tag, and `process_group` wraps the call in
    `try/except ValueError` so a bad shot joins `failed` and is reported alongside
    the mis-sized ones. Verified: a truncated-signature file now yields a
    `ValueError`, and a real generated PNG still parses.

- [x] R1.2 (MINOR) web/src/wiki.ts:264 - the section-icon `<img>` was appended to
  the live DOM immediately with `src` set and removed only `onerror`, so a
  not-yet-captured icon flashes a broken-image glyph before `onerror` fires -
  exactly the flash the sibling `upgradeFigures` in `site.ts` is written to
  avoid. Inconsistent, and it bites the next un-captured icon.
  - Response: FIXED. Now mirrors `site.ts`: the img is built detached and appended
    in `onload`, so a 404 leaves the hatched placeholder frame untouched with no
    flash. `npx tsc --noEmit` clean.

- [x] R1.3 (NIT) crates/nova_debug/src/harness.rs:308 - the doc comment on
  `REEL_CAPTURE_RESOLUTION` claimed "a packaging step downscales thumbnails from
  this", but the packaging script copies thumbnails as-is and the site sizes them
  down with CSS. Doc/reality mismatch.
  - Response: FIXED. Comment now says thumbnails share the capture and the site
    sizes them down with CSS at ~300px wide.

- [ ] R1.4 (MINOR) [left as-is] scripts/gen-web-screenshots.py aliases - an alias
  reads its source from `web/src/assets/` (already-copied), so if the source was
  not captured this run but a stale copy from a previous run exists, the alias
  silently propagates the stale image. Low impact given the regen workflow (a
  full run always recopies the source first); noting it rather than adding
  bookkeeping that the current single-run usage does not need.
</content>
</invoke>
