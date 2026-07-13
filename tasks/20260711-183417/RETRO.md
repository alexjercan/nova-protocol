# Retro: Audio SFX - thruster hum per-ship distance attenuation

- TASK: 20260711-183417
- BRANCH: fix/audio-hum-attenuation (landed as da5528f)
- REVIEW ROUNDS: 1 (APPROVE; 2 MINOR + 2 NIT, all addressed and verified)

## What went well

- The plan pass verified the mechanism from source (the code comment even
  documented the deferral) before any code was written, so implementation
  was mechanical and review round 1 found no correctness issues.
- Splitting the sink-coupled system into compute (resource) + apply (sink
  write) made the volume logic App-testable headless - AudioSink cannot be
  constructed without an audio device. Reusable pattern for any audio-volume
  logic that needs tests.
- The fresh-context review agent earned its cost: it re-derived the
  hierarchy claim from the actual SPAWN site (my TASK.md had cited a
  consumer query as evidence), caught the missing delivery guard on the
  "stays zero" test, and found the one real behavior delta (pre-sink
  smoothing) I had recorded as "unchanged".
- Sabotage A/B after committing the fix, with numbers recorded (4 of 6 new
  tests fail on pre-fix behavior; the 2 that pin shared behavior survive,
  as predicted in writing beforehand).
- The runtime trace ended up landing on the SHIPPED menu scene, which
  reproduced the reported bug with real numbers (hum 0.26/0.3 from 341 u).

## What went wrong

- Two example runs were wasted on trace vehicles that never burn inside
  their autopilot windows (13_menu_newgame clicks away in ~1 s;
  06_torpedo_range's script does not fire in time). Root cause: the rig was
  picked by scene CONTENT (has ships/torpedoes) instead of by script
  TIMELINE (when does the stimulus actually fire).
- The first trace run silently did nothing: xvfb-run does not exist on this
  host, and the grep/awk pipeline swallowed the 127 so the failure read as
  a clean empty run. Root cause: unverified host tooling plus a pipeline
  whose exit code came from the last stage.
- The intended pre-fix trace run got contaminated: I edited audio.rs while
  the cold worktree build was in flight, and cargo reads a crate's source
  when it COMPILES it (minutes into a cold build), not at launch. Recovered
  with a file-copy A/B, but the run was indeterminate and had to be redone.

## What to improve next time

- Before picking an example as a runtime-evidence vehicle, read its
  autopilot script and answer "at what second does the stimulus fire, and
  is that inside the window?" - or skip examples and use the plain app when
  the target state is an idle scene (the menu).
- `which <tool>` before the first long headless run on a new host, and make
  launcher failures loud (no bare `cmd | grep | awk` where the launcher's
  exit vanishes).
- Treat a launched cold build as OWNING the tree: no source edits until the
  evidence run completes, or copy the tree/file first.

## Action items

- [x] Ledger: bumped `out-of-context-review-pass`,
      `delivery-guards-on-null-assertions`, `diagnostic-first`; added
      `trace-vehicle-timeline-first`, `silent-tool-missing-in-pipeline`,
      `no-source-edits-during-inflight-builds`.
- [ ] The examples-rework task (20260712-211352) already carries the
      follow-up: pin this fix at example level where it adds coverage.
