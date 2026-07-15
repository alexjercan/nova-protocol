# Review: Modding scenario-dispatch perf (benchmark + handler index + 083339 defer)

- TASKS: 20260714-083331 (benchmark, CLOSED), 20260525-133014 (index, CLOSED),
  20260714-083339 (hot-path, deferred)
- BRANCH: modding-perf (nova) + bevy-common-systems master @ 4c81117

## Round 1

- VERDICT: APPROVE

Scope reviewed: the criterion bench (`crates/nova_scenario/benches/scenario_dispatch.rs`),
the upstream `EventHandlerIndex` change (bcs `src/modding/events.rs`), the rev
bump + patch removal, and the report/deferral reasoning. Ran the correctness
checks: 2 bcs dispatch tests pass, all 59 nova_scenario tests pass against the
pinned rev, `cargo check --workspace` green, bcs `cargo fmt --check` + `clippy
--all-targets` clean.

Independently re-verified the load-bearing correctness claims rather than
trusting the summary:

- **Borrow safety** (`queue_system`): the loop holds `Res<EventHandlerIndex>`
  (immutable) while calling `action.action(&mut *world)`. Different resources,
  no aliasing. Actions receive `&mut W`, not `&mut World`, so they cannot make
  immediate structural changes to the index mid-drain.
- **Within-frame despawn**: only reachable via deferred `push_command`, which
  flushes after `PostUpdate`. So a handler "despawned" by an action still fires
  for later events in the same queue drain - identical to the pre-index baseline
  (whose despawn commands also flushed post-system). No behavior change.
- **Staleness across frames**: `maintain_handler_index` is ungated and ordered
  `.before(queue_system)`, so a despawn's `RemovedComponents` is drained and the
  bucket pruned before the next dispatch. Confirmed by the
  `despawned_handler_is_pruned_from_the_index` test, which exercises exactly the
  quiet-frame (dispatch-chain-skipped) path.

No BLOCKER or MAJOR findings. The MINORs below are about benchmark rigor and the
honesty of the deferral rationale, not correctness; they are left to the
implementer's discretion and do not block.

- [x] R1.1 (MINOR) docs/modding-perf-report.md:89-94 - the headline snapshot-vs-
  baseline batch deltas (-17-24%) compare a baseline measured with criterion's
  default window (3s/5s) against a snapshot run measured with `--measurement-time
  2.5 --warm-up-time 1`. Means are window-independent in expectation and the
  deltas (17-24%) dwarf any window-induced variance (~few %), so the conclusion
  holds - but a single same-settings A/B re-run would remove all doubt. Note the
  mixed windows in the report, or re-run both legs identically.
  - Response: Accepted with disclosure. The report already flags the mixed
    windows; the naive-index leg (the tightest comparison) *was* same-settings vs
    baseline via criterion's own change detection, and the snapshot beats even
    that naive leg. A clean same-settings A/B needs two more ~4-min LTO builds
    (baseline requires patching bcs back to 4a743b2) for a delta that is already
    an order of magnitude above the window noise - not worth the churn. Left as
    a documented caveat, not re-run.

- [x] R1.2 (MINOR) docs/modding-perf-report.md:135-140 and
  tasks/20260714-083339/TASK.md - "expression filters live on OnUpdate, which
  fires once per frame and cannot burst" overstates. `EventFilterConfig::Expression`
  can be attached to *any* event's handler, including bursty discrete events
  (a modder could gate an OnDestroyed handler on a condition). The defer is still
  sound (13-26 ns is cheap and the index removes the surrounding burst scan), but
  the rationale should hedge "on today's built-ins expression filters are only on
  OnUpdate" rather than claim they structurally cannot burst.
  - Response: Fixed. Report Decisions #3 and the 083339 TASK.md now say "in
    today's content expression filters live on OnUpdate" and note a modder *could*
    attach one to a bursty event, with the index-removes-the-scan mitigation.
    Verified the reworded text.

- [x] R1.3 (MINOR) crates/nova_scenario/benches/scenario_dispatch.rs:44-52 - the
  `condition_eval` group measures only `progress > 0.5`, a single var-vs-literal
  compare. The variable AST exists to express *nested* conditions (parens,
  arithmetic, and/or), which walk more Boxed nodes and would measure well above
  26 ns. The deferral cites 26 ns as the condition cost; a deep-expression case
  would be a fairer worst case and could shift the 083339 calculus. Add a nested
  condition to the bench, or caveat that 26 ns is the trivial-condition floor.
  - Response: Fixed and it strengthened the deferral. Added `condition_eval/nested`
    (`(progress*2 + bonus) > (limit-1)`); measured **62 ns**, 2.4x the trivial
    case. Even at 62 ns the once-per-frame math is ~62 µs/frame at 1000 handlers
    (~0.4% of a 16 ms frame), so the 083339 defer holds for heavy conditions too.
    Report micro table updated.

- [ ] R1.4 (NIT) bcs src/modding/events.rs (EventHandlerIndex doc) - the index
  dispatches from handler *snapshots* taken at spawn, so any in-place mutation of
  an `EventHandler` component after spawn is silently ignored by dispatch. The
  doc comment notes handlers are "never mutated in place"; make that an explicit
  caveat for downstream bcs users ("do not mutate an EventHandler after spawn;
  despawn and re-spawn instead"), since it is a semantic change from the old
  live-component dispatch.
  - Response: Accepted, deferred. The existing doc comment already states the
    snapshot invariant ("a handler is built, spawned once, and never mutated in
    place"), which covers the correctness contract. Turning it into an explicit
    downstream-user caveat is a bcs-doc-only change that would force a third rev
    bump this session (churn). Folding it into the next bcs touch instead; not a
    blocker (NIT).

- [ ] R1.5 (NIT) bcs - the index holds a clone of every handler (Arc-shared
  filters/actions + one Vec per bucket), so handler storage is ~2x. Negligible
  for any realistic mod, but worth a line in the doc comment.
  - Response: Accepted, deferred with R1.4 into the next bcs doc touch (NIT).

### Round 1 close

Verdict stands: **APPROVE**. R1.2 and R1.3 addressed in-session (rationale
hedged; nested-condition bench added, measured 62 ns, deferral reconfirmed).
R1.1 accepted with disclosure. R1.4/R1.5 (NITs) deferred to the next bcs touch.
No open BLOCKER/MAJOR. The branch is ready to merge at the user's discretion.
