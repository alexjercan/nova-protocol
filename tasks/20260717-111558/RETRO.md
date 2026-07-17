# Retro: Mod-shipped skybox cubemaps bypass load-time meta (Always switch)

- TASK: 20260717-111558
- BRANCH: fix-mod-skybox-meta
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Verified the load-bearing objection against pinned bevy source before
  designing around it. The old `assets_plugin()` doc comment asserted `Always`
  "would fire an HTTP request per asset on wasm just to 404" and used that to
  justify the per-path `Paths` set. Reading three source hops
  (`server/mod.rs:1564`, `io/wasm.rs:100-124`, `server/mod.rs:1616`) showed the
  404 is real but NON-FATAL (graceful fallback to `default_meta`). That flipped
  the decision from "avoid Always" to "Always is fine" with confidence, and it
  was cheap - minutes of reading. Another `verify-engine-guarantees-in-source`
  occurrence.
- Review (same session, so a blind spot) independently re-derived the one claim
  most likely to have a hole: the fix's coverage of DOWNLOADED mods. The new
  test only exercises the shipped path; reading `nova_portal_gen`'s `walk_files`
  ("every file verbatim", sidecar included) + `mod_binary_resources.rs:145`
  confirmed the downloaded path is covered too. That is exactly the "re-verify a
  load-bearing claim the diff does not test" the review skill calls for.
- Caught `cargo fmt --all` reformatting five unrelated example files (pre-existing
  indentation drift) and reverted them, keeping the commit scoped.

## What went wrong

- Spent ~10 min on a `trunk build` that could not actually verify the thing in
  question. The user asked to "start the wasm build to check" the 404 - but 404s
  are a RUNTIME behavior, invisible to a build. The build only confirmed `Always`
  compiles. The real proof was the source read. Root cause: a build is the wrong
  instrument for a runtime claim. It was not wasted (the user asked, and
  compile-confirmation has some value), but the verification method should match
  the claim's nature - runtime -> source or a runtime harness, not a build.

## What to improve next time

- When a claim is about runtime behavior (a 404, a per-frame cost, an ordering),
  reach for source-reading or a runtime probe first; use a build only to answer
  "does it compile". State that distinction up front so the instrument matches
  the question.

## Action items

- [x] Filed tatr 20260717-133332: editor's direct `SkyboxConfig` insert may miss
  its `Cube` view (pre-existing, surfaced in review R1).
- [x] Bumped `verify-engine-guarantees-in-source` in the ledger with the
  own-doc-comment-asserts-a-cost variant.
- Process detail and mechanism live in TASK.md (Decision) and
  docs/design/mod-skybox-meta-always.md; not repeated here.
