# Review: Retire the diegetic objective reveal card

- TASK: 20260729-211200
- BRANCH: feat/objective-chip-is-the-posting

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

The out-of-context reviewer ran `cargo check --workspace --all-targets` (clean,
no nova-crate warnings), `cargo test -p nova_gameplay --lib -- objective_stack::
objective_feedback::` (14 passed, 0 failed - 10 stack + 4 feedback) and DoD 3's
grep (no output, exit 1). It did not re-run the probe (~10 min) but read the
run's artifacts: `probe-runs/b2a7b76c/playable/checks.json` matches the verdict
TASK.md records, and `run.log` contains `ObjectiveStackPlugin` with zero
`ObjectiveReveal` hits, so the run did exercise the card-less tree. It checked
each deleted test against `deleting-a-test-salvage-live-assertions` and found
the salvage sound: both replacement tests are revert-sensitive (under the old
gate, `first` would still be unread at 12.6 s because its pop only started at
the 3.2 s fallback, and the re-worded chip would still be up).

In-session re-derivation of the round's load-bearing claim (R1.1): grepping
`web/ README.md crates/` for the card's PROSE rather than its symbols -
"cockpit moment", "slightly rotated", "tucks into", "as the card" - returns
`web/src/wiki/hud.md:66` and nothing else. Confirmed: the symbol-only sweep did
miss it.

Pending user check, not resolved by this verdict:

- DoD 5 (manual): owner playtest that a posted objective shows ONLY as the
  top-centre chip, arriving like a chat notification, with no cockpit card
  first. This is also where the shortened total on-screen time gets judged -
  12 s from the posting, where it used to be 3.2 s of card plus 12 s of chip.

- [x] R1.1 (MAJOR) web/src/wiki/hud.md:66 - the doc sweep fixed the
  "Contextual HUD" bullet at line 31 but missed the *Comms and objectives*
  section, which still describes the deleted card in full ("A newly posted
  **objective gets the cockpit moment**: it appears slightly rotated on the
  HUD, holds ... then tucks into the **objective stack** ... A chip pops as the
  card lands"). Live player-facing page, and Step 5 is ticked as done. Missed
  because the sentence never uses the words `reveal` or `card`-the-module, so a
  symbol-only grep does not reach it (`keep-docs-in-sync-with-code`, x8).
  Rewrite it to match line 31 and the CHANGELOG, then re-grep the doc tree for
  the card's prose to confirm no other hit survives.
  - Response: fixed. The sentence now reads "A newly posted **objective**
    arrives the same way, in the **objective stack** at the top of the screen
    ... A chip pops the moment its objective posts and then breathes quietly."
    Re-grepped `web/ README.md crates/` for "cockpit moment", "slightly
    rotated", "tucks into" and "as the card": zero hits. The wider sweep that
    found this also turned up `web/src/tutorial.html:85` ("the objective panel
    just states the goal"), which names the panel retired back in
    20260724-134312 - PRE-EXISTING drift, so it is filed as task
    20260730-111146 rather than folded into this branch.

- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/nova_os.rs:1255 - three source
  comments still describe deleted machinery (`doc-sweep-covers-source-doc-
  comments`): the anchor parenthetical at 1255, "the reveal's tuck anchor now"
  at 3161, and a 5356 comment pointing at
  `the_stack_publishes_the_reveal_tuck_anchor`, a test that no longer exists.
  - Response: fixed, all three. The 1255 parenthetical is dropped; 3161 now
    reads "the top-centre objective stack carries the NOVA OS affordance now -
    its TAB keycap footer"; the 5356 pointer no longer names a dead test ("the
    tab handle's own tests went with it in task 20260724-134312; the stack that
    replaced its affordance is tested in `objective_stack`"). The remaining
    `reveal` hits in the file are the NOVA OS boot banner's row reveal,
    unrelated to the card.

- [x] R1.3 (NIT) crates/nova_gameplay/src/hud/objective_stack.rs:308 - `let age
  = shown.age_secs;` is a leftover shim from the old `let popped_for = *pop;`
  line, aliasing a field read one line above. Inline it.
  - Response: fixed - the match arm reads `shown.age_secs` directly, and the
    comment above it stays as the explanation for why the dwell counts from the
    posting.

Re-verified after the fixes: `cargo fmt --all --check` clean; `cargo check
--workspace --all-targets` clean with no nova-crate warnings; `cargo test -p
nova_gameplay --lib -- objective_stack:: objective_feedback:: nova_os::` -> 77
passed, 0 failed (objective_stack alone: 10 passed, so no filter matched zero -
`validate-proof-command-shape-at-plan-time`). `tatr check --ledger LESSONS.md`
exits 0.


## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context (round-1 reviewer, resumed against the fix diff)

Re-ran on `eee136f2`: `cargo check --workspace --all-targets` clean of any
nova-crate warning or error; `cargo test -p nova_gameplay --lib --
objective_stack:: objective_feedback:: nova_os::` -> 77 passed, 0 failed; DoD
3's grep still exits 1 with no output. Re-ran the round-1 prose sweep
independently over `web/ README.md CHANGELOG.md crates/` for "cockpit moment",
"slightly rotated", "tucks into", "as the card", "tuck anchor" and
"the_stack_publishes": zero hits, so R1.1 and R1.2 are closed rather than
reworded around. Round 2's only code edit is the R1.3 shim removal, a pure
refactor that changes no behaviour, and it introduces nothing new.

The reviewer agreed with filing `web/src/tutorial.html:85` as task
20260730-111146 rather than folding it in - the sentence was stale before this
branch touched anything. It flagged that that task's DoD 1 grep has exactly one
hit, so it could be satisfied by a single-sentence edit while its second step
(sweep the rest of the unswept page) goes unproven; that is the new task's
problem, and its DoD has been strengthened accordingly.

Still open, and not a review matter: DoD 5, the owner playtest.
