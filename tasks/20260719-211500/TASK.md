# Probe surface close-out: remove deprecated sweep|web|profile aliases + trace verb; keep perf_web (the wasm web-capture app) with sharpened docs; AGENTS/skill/wiki sweep

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.8.0,tooling,refactor

## Goal

Close out the probe surface (user request 2026-07-19, queued after the
probe-all T1/T2/T3 family):

- REMOVE the deprecated `sweep|web|profile` alias subcommands (their
  CHANGELOG note promised one release cycle; the v0.8.0 cut is it).
- REMOVE the `trace` verb: redundant - `--profile` renders the top-N
  table into the run report, and `probe report <dir>` re-renders it from
  the run dir's trace.json. The only lost capability is tabling a
  chrome trace that came from OUTSIDE a probe run dir; nobody does that.
- perf_web bin: KEEP - investigated 2026-07-19 while filing this task:
  `perf.html` declares `data-bin="perf_web"`, so perf_web IS the wasm
  app `--platform web` trunk-builds and drives in Chromium (the wasm
  half of the web capture, not a CLI leftover). Removing it removes the
  web capture. Instead: sharpen its doc header + the probe.rs Platform
  doc so the next reader does not mistake it for dead code (this task's
  filing is the second time it looked removable).
- Docs sweep: AGENTS.md probe lines, the probe skill (drop the alias
  sentence + trace command; add whatever T1 landed), wiki Performance
  section, CHANGELOG Unreleased (aliases + trace removed; perf_web
  clarified).

## Steps

- [ ] Delete the three alias arms + `Cmd::Trace` + trace_table from
      probe.rs; USAGE text + doc header updated; parse pins for the
      removals (unknown subcommand error now names run|report only).
- [ ] perf_web doc header: state its role (the wasm capture app
      perf.html builds for `--platform web`) and why it is a [[bin]].
- [ ] Docs sweep (skill, AGENTS.md, wiki, CHANGELOG); re-grep
      `probe sweep|probe web|probe profile|probe trace` - only history
      remains.
- [ ] Verify: fmt; cargo test -p nova_probe; wasm check (perf_web still
      builds).

## Notes

- Queued behind T1 20260719-210438, T2 20260719-210443, T3
  20260719-210450 by user direction ("queue this after you are done
  with the current 3 T's").
- If the user still wants perf_web GONE knowing it kills
  `--platform web`, that is a scope decision to re-ask - not assumed
  here.
