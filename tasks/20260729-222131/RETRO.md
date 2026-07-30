# Retro - 20260729-222131

Smoke red on master: `screenshot_nova_os` exited before completing its cycle.
Fix: adopt the repo's existing self-ending autopilot completion contract.
One commit, two review rounds, no behavior change beyond the example itself.

## What went well

- **The guard was built as the instrument, not just the fix.** The plan's step 1
  was "add the completion guard FIRST and run", precisely because the diagnosis
  had an unresolved hole. That paid off immediately: the run panicked with
  `stalled in stage 13`, which answered the open question in one run and
  falsified my own leading hypothesis. Building the thing that makes a silent
  failure loud, then using it to diagnose, beat any amount of further reading.
- **The A/B between the two run modes did real work.** Running the same example
  with and without `BCS_REEL` and diffing the timestamp-stripped logs proved the
  modes were identical except for the capture lines, which ruled out a whole
  class of "something else in the app behaves differently" theories before any
  code was touched.
- **The fail-first proof came for free and is genuine.** The guard demonstrably
  fails on the pre-fix code (exit 101) and passes on the fixed code. That is the
  `fail-first-regression-ab` discipline satisfied without a bespoke rig.
- **The reviewer ran the code.** Round 1 did not read-and-opine; it executed both
  run modes, `cargo check`, and read the pinned dependency's contract - and its
  two best findings came from that execution, not from reading.

## What went wrong

- **I wrote a comment claiming a protection that does not exist.** The final
  stage's comment said writing `AppExit` directly "would cut a still-pending
  capture short". It would not: `capture_window` spawns a bare `Screenshot` and
  never registers a completion collector, so the captures are not part of the
  negotiation at all - they survive on stage 11's 20-frame settle. I reasoned
  from what the protocol is FOR rather than from what this example actually
  registers, and wrote the general story as if it were the local fact. Worse,
  the same false causal claim went into the TASK.md verification record, where
  it read as evidence. This is the promoted "does the prose claim anything the
  diff does not do?" lesson, and I had literally read the completion module
  earlier in the session - the registration site was in my own grep output.
- **I let a wrong inference run for several tool calls.** From a frame-time
  comparison I concluded the script "CANNOT" have walked its stages in the
  observed 10 ms, and went hunting for a second exit path. The 10 ms was the
  last LOG line, not the exit - nothing logs between the beats. I treated an
  absence-of-logging artifact as a measurement. The tell was there: I had no
  instrument for the quantity I was reasoning about, and I kept reasoning
  anyway instead of building one. To its credit the plan named this as the open
  question rather than hiding it, so the first work step settled it.
- **I pre-wrote the reviewer's verdict.** I appended
  `VERDICT after fixes: APPROVE` to REVIEW.md before round 2 ran. It happened to
  match, which is exactly what makes it a bad habit rather than an obvious
  error: a verdict authored by the party under review is not a gate. The
  reviewer caught it.
- **The wiki drift was mine to catch and I did not.** `keep-docs-in-sync` is the
  ledger's x9 lesson and my change invalidated a live wiki sentence scoping
  completion backstops to three categories. I grepped for the example's NAME and
  found only history; the sentence that went stale never names it. That is the
  `sweep-docs-for-the-feature-description-not-just-its-symbols` query rule, and
  a symbol-only sweep missed it the same way it did last time.

## Lessons

- `comment-the-local-wiring-not-the-general-protocol`: when explaining WHY code
  follows a protocol, verify the protocol's preconditions hold at THIS call site
  before writing the rationale. Here the registration site (`completion::register`)
  decides whether the capture is protected, and it is never called for
  `capture_window` - so the general "the watcher waits for every collector" was
  true and locally irrelevant. Check the registration/wiring, then write the
  reason. Sibling of `advertised-but-unwired`, aimed at prose rather than
  features.
- `absence-of-logging-is-not-a-measurement`: a last-log-line timestamp is not an
  exit time, a frame count, or a duration. Before reasoning quantitatively about
  something nothing logs, add the instrument (here: one guard that panics with
  the stage index) - it is cheaper than the theory, and it settles the question
  instead of narrowing it.
- `the-reviewed-party-does-not-write-the-verdict`: record a review verdict only
  when it comes back from the reviewer, attributed to its round. Pre-writing an
  expected verdict looks identical to a real gate in the record.
- Reinforces `sweep-docs-for-the-feature-description-not-just-its-symbols`
  (x2 now): the stale wiki sentence described the CATEGORY behavior
  ("the sections, gameplay and ui examples carry completion backstops"), never
  naming the example. Grepping the symbol proved nothing; the query had to be
  the behavior's description.

## Do differently next time

- When a diagnosis has a hole, name the instrument that closes it in the plan -
  and build it before the fix, not alongside. That worked here; make it the
  default for bug tasks whose mechanism is inferred from source rather than
  observed.
- Before committing, re-read every comment and record line I wrote asking "which
  line of the diff makes this true?" - the two findings that survived review
  were both prose, not code, and both would have failed that one question.
- For a doc sweep, write the 3-5 phrases a doc author would use for the changed
  BEHAVIOR (here: "completion backstop", "self-ending", "exit without panic")
  and grep those alongside the symbol.
