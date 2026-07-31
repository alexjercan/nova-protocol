# Review: KISS: nova_gameplay HUD - chrome and objective surfaces

- TASK: 20260731-170335
- BRANCH: refactor/kiss-hud-chrome

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

No BLOCKER. One MAJOR and four MINOR/NIT. All were addressed, but the fixes
themselves introduced findings, so the round closes REQUEST_CHANGES and
round 2 carries them. The in-session pass re-derived R1.1, R1.2 and R1.3 from
the tree before accepting them, and re-ran `cargo fmt --check` (exit 0),
`cargo check --workspace --all-targets` (exit 0) and `cargo test --lib -p
nova_gameplay` (785 passed) after the fixes.

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/key_glyphs.rs:25 - the
  deferred-work reference lost its tatr ID (`the remapping/gamepad follow-up
  (20260710-231927)` -> a bare `NOTE:`), so it now points at nothing, while
  the parent epic's rubric says deferred work is kept as
  `TODO:`/`FIXME:`/`BUG:` WITH the tatr ID if one exists - and 20260710-231927
  is still OPEN. Restore it as `TODO(20260710-231927):`.
  - Response: confirmed and fixed - restored as `TODO(20260710-231927):`.
    Re-derived the premise before accepting: 20260710-231927 is STATUS OPEN,
    and the epic rubric's deferred-work row does say "with the tatr ID if one
    exists". The demotion to a bare `NOTE:` was wrong. DoD 3 now returns
    exactly this one hit, which it permits when NOTES.md lists it; NOTES's
    "DoD 3" section was rewritten to do so. Reviewer re-checked and confirmed.
- [x] R1.2 (MINOR) tasks/20260731-170335/NOTES.md:104 - the measured figure
  "143 of the removed lines carry a tatr ID, a bare `YYYY-MM-DD` date, a
  `review Rn.n` clause or a record pointer" does not reproduce: those four
  categories over `git show -U0 8eabd5d5 -- crates/` count 140. Correct the
  number to the one the stated pattern produces.
  - Response: confirmed and fixed. Re-counted: the four listed categories give
    140; 143 was the count WITH `playtest round` in the pattern, which the
    sentence did not name. NOTES now states 140 for the listed categories, the
    143 variant with its extra term, and the exact command. Reviewer
    re-confirmed 140.
- [x] R1.3 (NIT) tasks/20260731-170335/NOTES.md:9 - the stated method
  "measured at the first `#[cfg(test)]` in each file" does not produce the
  table's own `mod.rs` row (first `#[cfg(test)]` is line 47, which would give
  46/1417, not 964/499); 14/15 rows reproduce. Restate the method as the
  `mod tests` boundary.
  - Response: confirmed and fixed. `mod.rs`'s two `#[cfg(test)] mod <rig>;`
    declarations at 47 and 52 sit above `mod tests`, so "first `#[cfg(test)]`"
    was the wrong description of what was measured. NOTES now says the
    `mod tests` boundary and names mod.rs as the exception.
- [x] R1.4 (NIT) crates/nova_gameplay/src/hud/objective_feedback.rs:1 - the
  substitution left orphan half-lines where a clause was cut without
  rewrapping ("...silently, so" / "completions"); same in beacon_chips.rs:1,
  keybind_dock.rs:8, objective_markers.rs:1, objective_stack.rs:1,
  holo_instruments.rs:1, flight_status.rs:1. Rewrap those blocks to the file
  fill.
  - Response: fixed in all seven blocks. The re-check found two paragraphs
    still short mid-paragraph (`objective_markers.rs:5`,
    `keybind_dock.rs:14`); both rewrapped. This fix is what caused R2.1 -
    three files lost a line, falsifying NOTES's measured table.
- [x] R1.5 (NIT) crates/nova_gameplay/src/hud/holo_instruments.rs:9 - "when
  the arrival solve becomes gravity-aware a curved prediction can replace it"
  lost its pointer (20260710-193500, now CLOSED), so a future reader has no
  way to notice the precondition may already have shipped. Add a `TODO:` or
  drop the forward-looking clause.
  - Response: declined as a code edit, filed as backlog 20260731-232634
    instead. Deciding whether the ribbon doc is stale means reading the
    autopilot's arrival solve, and the epic's Out of Scope says any defect
    found becomes a backlog task, not a fix in this commit. NOTES's "Defects
    found" records it. Reviewer accepted this as the correct answer.

Process signal: the branch carried two commits (d2c3d333, 13da1583) whose
patch-ids match master's fa0e227a/ac70dba8 - the link-RAM fix, landed to
master separately. `git diff master...branch` therefore overstates the
change; the task's own commit is 8eabd5d5. Rebase before merge so the history
is not duplicated. Left as-is here because the task stops at REVIEWING.

Process signal: `tatr proofs 20260731-170335` returns an empty list (exit 0)
despite six DoD proof lines, so the reviewer had to enumerate the proofs by
hand. Worth a look at whether the proofs parser handles this DoD block shape.

Verified independently in-session: the comment-only claim holds - the entire
non-comment change under `crates/` is one deleted blank line, so no
executable line moved and no test was added, weakened, renamed or deleted.
DoD 1 check green, DoD 2 fmt clean, DoD 4 line counts all reproduce with
`keybind_dock.rs` (1911) the only file over 1500 and its shared-sizing
justification real (`keycap_sizing_tests` does `use super::tests::{...}`).
DoD 3's grep returns exactly the one deliberate `TODO(20260710-231927)` the
R1.1 fix restored, which is what DoD 3 permits when NOTES lists it - and
NOTES does. `cargo doc` warnings are unchanged from master, with no new
dangling or private intra-doc link.

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

Round 2 findings are the round-1 fixes' own fallout, all in the records. The
same out-of-context reviewer raised them while re-checking round 1; the
in-session pass re-measured every figure itself before accepting, and re-ran
fmt, check and the lib suite on the fixed tree.

- [x] R2.1 (MAJOR) tasks/20260731-170335/NOTES.md:14 - the R1.4 reflow
  shortened `flight_status.rs`, `objective_feedback.rs` and
  `objective_stack.rs` by one line each, so the structure table no longer
  reproduces (actual 551/338, 530/281, 1049/551 against a table saying
  552/339, 531/282, 1050/552) and three marker-inventory line numbers moved.
  Re-run `wc -l` and the marker grep and update both tables.
  - Response: confirmed by re-measuring all 15 files and all 12 markers
    myself, after the two remaining R1.4 rewraps so the numbers are final.
    Both tables corrected: three Lines/Prod cells, and
    `objective_feedback.rs:14 -> 13`, `objective_stack.rs:434 -> 433`,
    `602 -> 601`. The other 12 rows and 9 markers were unmoved.
- [x] R2.2 (MINOR) tasks/20260731-170335/TASK.md - the close-out's "217
  insertions, 245 deletions across 134 hunks" describes 8eabd5d5 alone; with
  the fixes the branch totals are different. Restate whichever form is
  actually committed.
  - Response: confirmed. The branch total over `crates/` is 240 insertions /
    271 deletions across the same 134 hunks. Both TASK.md and NOTES.md now
    give the branch figure and name the range it was measured over, with the
    pass commit's 217/245 kept as the sub-total it is.
- [x] R2.3 (NIT) tasks/20260731-170335/NOTES.md:99 - "eleven `NOTE:`, one
  `TODO:`" is slightly off: the marker is written `TODO(20260710-231927):`,
  so a plain `grep 'TODO:'` sweep misses it. State the parenthesized form.
  - Response: fixed - NOTES now gives the parenthesized form and says
    explicitly that a bare `grep 'TODO:'` will not find it.

Re-verified in-session after the round-2 fixes: `cargo fmt --check` exit 0,
`cargo check --workspace --all-targets` exit 0, `cargo test --lib -p
nova_gameplay` 785 passed / 0 failed / 1 ignored. All 15 `wc -l` values and
all 12 marker line numbers now reproduce against the tree. DoD 3's grep still
returns exactly the one listed `TODO(20260710-231927)`. The comment-only
claim holds unchanged: the entire non-comment change from `8eabd5d5^` to the
tree is one deleted blank line, and no test was touched.

Pending user checks (do not block APPROVE):

- DoD 6 (`manual:` owner skims the diff and agrees no behavior changed).
- 20260731-232634's Done Means is a `manual:` owner check by design.
