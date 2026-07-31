# Condense AGENTS.md into a repository routing guide

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, docs, process
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

As a contributor, I want a concise AGENTS.md that routes me to the current
code, workflow, and documentation without repeating their full contents.

## Steps

- [x] Verify current crates, commands, workflow records, and documentation
      pointers against the repository.
- [x] Rewrite AGENTS.md using fragments, bullets, flat structure, and tables.
- [x] Preserve repository-specific safety, testing, generated-content, docs,
      and shared-checkout rules.
- [x] Re-read the final document and run focused documentation checks.

## Definition of Done

- AGENTS.md is materially shorter than its 297-line, 2,519-word baseline.
  (cmd: `wc -l -w AGENTS.md`)
- Workspace crate names and critical local commands remain accurate.
  (manual: compare AGENTS.md with root Cargo.toml and repository scripts)
- Required workflow pointers and hard rules remain discoverable.
  (cmd: `rg -n 'LESSONS.md|tatr|sprout|nix develop|cargo test|generated|shared checkout|keeping-docs-in-sync' AGENTS.md`)
- Markdown has no non-ASCII punctuation or malformed structure.
  (cmd: `LC_ALL=C grep -n '[^ -~]' AGENTS.md` returns no matches)

## Notes

- Documentation-only change. No Rust test suite required.
- Prefer pointers to durable wiki/task/skill sources over copied procedures.

## Close record

- Replaced the 297-line, 2,519-word guide with a 173-line, 1,085-word routing
  document.
- Preserved crate map, commands, harness-first testing, generated content,
  shared checkout, tatr flow, task scheduling, docs sync, and release pointers.
- Corrected two stale inherited facts against source: reference harnesses live
  under `crates/nova_assets/tests/`; `GameAssetsStates` includes `Boot`.
- Verified all workspace members, referenced paths, state/CLI source symbols,
  ASCII-only content, and `git diff --check`.
- Full Rust tests and clippy not run: documentation-only change; project policy
  reserves the full suites for CI.
