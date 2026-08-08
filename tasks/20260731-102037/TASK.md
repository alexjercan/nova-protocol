# lessons: fold 7 promoted ledger lessons into the work/review skills

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, chore, process

## Story

As the maintainer, I want the six x3+ ledger lessons that all name the SAME
target (prose in the work/review skills) folded into those skills in one pass,
so that the guidance lives where the agent actually reads it instead of
recurring as a ledger entry nobody acts on.

The skills live outside this repo (`~/.claude/skills/{work,review}`, managed in
nix.dotfiles), so the edit lands there; this task tracks the decision and the
proof.

## Steps

- [ ] work skill, verify step: "read each deleted test's ASSERTIONS and re-home
      the survivors" as a standing step of any diff that removes tests
      (`deleting-a-test-salvage-live-assertions`, x3).
- [ ] work skill, edit guidance: "anchor an insert on the DOC BLOCK start, not
      the `#[test]`/`const`/attribute line, and re-read the produced text around
      both items" (`anchor-edits-in-the-right-scope`, x3).
- [ ] work verify step + review Tests dimension: "when a change adds N symmetric
      callers or registration sites, pin each end-to-end in the SAME pass -
      enumerate the CALL sites, not the helpers"
      (`pin-each-caller-not-just-shared-core`, x4).
- [ ] work skill, verify step: "grep each intended module/test name in the
      output; re-run any absent one alone - and for an absence proof, grep the
      CLAIMS that were really in the tree, checking at plan time that zero is
      reachable" (`validate-proof-command-shape-at-plan-time`, x4).
- [ ] work skill, verify step: "run per-crate `cargo test -p <crate> --no-run`
      on touched crates before trusting a workspace check"
      (`match-ci-feature-set-in-targeted-tests`, x3).
- [ ] work skill, rig guidance: sharpen "grep the module for an existing rig
      first" to "copy the nearest passing rig WHOLE, then mutate - and the
      nearest rig may be the DEPENDENCY'S own, e.g. bevy_ui's
      `setup_ui_test_app`" (`reuse-known-good-stack`, x9, positive).
- [ ] work + review skills: content-wide behaviour changes check "base AND
      webmods AND assets/mods AND Rust-coded scenarios", and an asset
      rename/move sweeps every content-shaped file repo-wide (examples/,
      `include_str!`, test data) (`sweep-content-repo-wide-not-just-assets`, x3).
- [ ] Mark each of the seven entries SHIPPED in LESSONS.md with this task ID and
      the skill file it landed in.

## Definition of Done

- All seven lessons carry a PROMOTE disposition naming this task, and the
  ledger lints clean (cmd: `tatr check --ledger LESSONS.md`).
- Each of the seven prose lines is present in the named skill file
  (cmd: `grep -n "re-home the survivors" ~/.claude/skills/work/SKILL.md` and
  the equivalent grep per lesson - one grep per bullet above, each hitting).
- LESSONS.md records the SHIPPED annotation per entry with this task ID
  (cmd: `grep -c 'SHIPPED 2026-.*20260731-102037' LESSONS.md` -> 7).

## Notes

- Decision pass recorded 2026-07-31 via `tatr ledger --disposition`. The other
  two pending entries were resolved without a task:
  `lint-gate-is-the-last-step` ABSORBED by `.githooks/pre-commit`, and
  `new-required-system-param-sweeps-all-rigs` DEFERred at x1 (it sat under
  Pending promotions below the 3+ bar).
- Precedent for a batched promotion pass: 20260720-220051.


## Dropped

- REASON: ledger is not a thing anymore
