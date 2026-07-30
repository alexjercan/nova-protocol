# Review - 20260729-222131 (branch fix/nova-os-smoke-completion)

## Round 1 - out-of-context reviewer, 2026-07-30

- VERDICT: REQUEST_CHANGES

Reviewed commit 36c7af69 against master 92c153a8. The reviewer read the diff,
the task record, AGENTS.md, LESSONS.md, the three sibling self-ending examples,
the contract in the pinned `bevy-common-systems`
(`debug/harness/autopilot.rs`, `completion.rs`) and `tests/examples_smoke.rs` -
and RAN the example both ways under Xvfb rather than trusting the record:
smoke mode exits 0 with the sentinel and the collector chain, capture mode
exits 0 with all four PNGs saved before the sentinel.

Contract adoption itself was found correct and faithful to the siblings.
Confirmed independently: the failure class does not recur unpinned - the only
other examples writing `AppExit` directly are `widget_zoo` and
`nova_os_rtt_poc`, both in `NOT_SMOKED` with recorded reasons.

No BLOCKER, no MAJOR.

### R1.1 (MINOR, FIXED) - the final stage's comment claimed a protection that does not exist

`examples/screenshots/screenshot_nova_os.rs` justified reporting done rather
than writing `AppExit` by saying a direct write "would cut a still-pending
capture short". It would not: `capture_window`
(`crates/nova_debug/src/harness.rs:453`) spawns a bare
`Screenshot::primary_window()` with a `save_to_disk` observer and never calls
`completion::register`, and this example does not add `nova_screenshot()`. The
reviewer measured it - `all collectors done, exiting` fires ~10 ms after the
autopilot reports done, i.e. on the same frame the old direct write would have
ended on. The captures survive because stage 11 sets `wait = 20` frames, not
because of the protocol.

This is the promoted "does the prose claim anything the diff does not do?"
lesson, in the file the next person copies the pattern from. Fixed: the comment
now states the real rationale (the smoke sentinel, plus the protocol rule that a
registered collector reports done instead of writing `Success` itself) and
attributes the capture margin to the 20-frame settle. Not adding
`nova_screenshot()` - that would be a real behavior change beyond this task, and
the settle is the existing, working arrangement.

### R1.2 (MINOR, FIXED) - live wiki drift introduced by the commit

`web/src/wiki/dev/development.md` scoped completion backstops to "the sections,
gameplay and ui examples", with "the screenshot examples drive the shipped
scenes to capture frames" as the contrast. As of this commit a screenshots
example carries a backstop, so the sentence is wrong. AGENTS.md requires the
wiki fixed in the SAME task and `keep-docs-in-sync-with-code` is the ledger's
x9 lesson. Fixed. Also folded in two pre-existing omissions in the same two
list items the fix touches: `screenshot_nova_os` was missing from the
`screenshots/` list and `lifeline` from the `gameplay/` list.

### R1.3 (MINOR, FIXED) - a causal claim in the task record the diff does not support

`TASK.md` verification item 2 said "the harness watcher, not a bare `AppExit`,
now owns the exit" as the reason the saves land first. Same error as R1.1 - the
ordering did not change. Fixed: the causal clause is gone and the measured
margin is recorded instead.

### R1.4 (NIT, FIXED) - the guard's doc comment overstated its reach

"A premature `AppExit` - from anywhere -" is not quite true: the guard runs in
`Last`, unordered against bcs's private `completion_watch` (also `Last`), and
Bevy stops after the update in which the exit is written, so an exit written by
a later-scheduled `Last` system would go unobserved. Harmless for the class
actually targeted (the script's own `PreUpdate` write), but the comment now says
so instead of claiming universal reach.

### R1.5 (NIT, NOT CHANGED) - runway-expiry wording

`main()` says a runway expiry "is an error exit naming it" while the guard says
an unfinished `AppExit` panics; in practice the autopilot writes
`AppExit::error()` in `PreUpdate` and the `Last` guard then panics (exit 101),
so the panic is the observed ending. Left as is: the wording is verbatim from
all three siblings, and diverging here would make the four copies disagree for
no behavioral gain. R1.7's follow-up is the right place to settle it once.

### R1.6 (NIT, NOT CHANGED) - the panic names the not-yet-started stage

`script.stage` is read after the increment, so the message says "stalled in
stage 13" - an index no match arm has. Verbatim from `broadside`; changing it
here alone would break the copies' symmetry. Folded into R1.7's follow-up.

### R1.7 (NIT, FILED AS A TASK) - fourth verbatim copy of the guard

`guard_script_completion` is now duplicated across four examples, and
`tasks/20260719-235305/SPIKE.md:57` already proposed promoting it to protocol
level in `bevy-common-systems`. Out of scope for a red-smoke fix; filed as its
own backlog task rather than widened into this branch.

### R1.8 (NIT, DECLINED) - no CHANGELOG entry

Judgement call, and the call is no. The Unreleased "Internals & Tooling"
entries are capabilities (probe sandboxing, the fmt hook, the asset preload
pass); repairing one example's completion contract so a red suite goes green is
below that bar and would read as noise in the release notes. The task record and
the example's own header carry the detail.

## Round 2 - same reviewer, 2026-07-30

Re-reviewed the fixes against the source (confirming independently that the
autopilot IS a registered collector at `autopilot.rs:197` while `capture_window`
registers nothing, so the corrected R1.1 comment is true line-for-line), and
re-ran `cargo check --example screenshot_nova_os --features debug` and
`cargo fmt --check` clean. Both `.rs` hunks are comment-only, so round 1's
measured behavior stands. Independently re-measured the capture margin at
0.566 s against my 0.53 s - consistent. Agreed with the R1.5/R1.6 deferral and
the R1.8 decline, and found REVIEW.md's record of round 1 accurate.

Four residual NITs, all applied:

- The wiki's new sentence ran to 124 chars in a file wrapped to ~72-78, and the
  screenshots-list rewrap left a dangling short line. Both rewrapped.
- `screenshot_nova_os` was folded into the group parenthetical "drive the
  shipped scenes", which it does not do - it boots a one-ship range and drives
  the NOVA OS terminal. Given its own parenthetical, as every other
  differing example in these lists has.
- The R1.4 fix still overclaimed in miniature ("before **or during** that
  schedule"): an exit written by a `Last` system scheduled after the guard is
  exactly the unobserved case. Now says "before that schedule ... but not one
  written later within `Last` itself".

One process finding, accepted: this file originally ended with a
`VERDICT after fixes: APPROVE` line written by the implementer BEFORE the
round-2 re-review ran. It happened to match, but a verdict authored by the
party under review is not a gate. Replaced with the reviewer's actual verdict
below, attributed to the round it came from.

- VERDICT: APPROVE

