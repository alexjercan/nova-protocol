# Retro: unified nova_probe run-harness spike

- TASK: 20260719-112011
- BRANCH: feature/perf-html-report (spike + review + revisions; squash-landed)
- REVIEW ROUNDS: 1 adversarial + user adjudication -> APPROVE

## What went well

- The questionnaire-driven forks (correctness mechanism, profiler, crate
  shape, verdict automation) got real user decisions BEFORE the doc was
  written, so the revision round changed mechanisms, not the goal.
- The adversarial self-review deliberately re-derived every claim against the
  codebase and Bevy's APIs instead of trusting the just-written prose - and
  found a factual API error (M1), a measurement-contamination flaw (M2), and
  a sequencing hazard (M3c: goldens vs the queued campaign-polish tasks).
- The user's product instinct ("I don't like goldens that much") aligned with
  the review's technical case against them - the doc gave the user the
  evidence to make that call confidently, which is what a spike is for.

## What went wrong

- The spike asserted a dependency capability from recall ("top-N from Bevy's
  per-system diagnostics") - `SystemInformationDiagnosticsPlugin` is OS
  CPU/mem, and per-system timings only exist as trace spans. Root cause:
  wrote the mechanism without grepping Bevy or the repo (no trace feature is
  wired anywhere, which one grep showed).
- The seeded priorities inverted the dependency chain (T5 p55 above its
  dependencies) because they were assigned per-task "importance" instead of
  topologically; tatr has no dependency field, so priority order IS the only
  machine-readable order.
- The correctness fork converged on goldens without naming deterministic
  replay or invariant assertions as candidates - the divergence step was
  narrower than the doc's confidence implied.

## What to improve next time

- A spike stating a dependency's API capability cites the verifying
  source/grep in the doc, exactly like plan steps do (verify-first-plan-steps
  already covers plans; spikes are not exempt).
- When seeding a task family, assign priorities strictly descending along the
  dependency chain so a naive priority-order picker executes a valid
  topological order.
- For a design fork, enumerate the standard alternatives of the DOMAIN
  (replay, goldens, invariants, property checks for correctness-testing)
  before scoring, not just the two that came to mind.

## Action items

- [x] Revisions applied to SPIKE.md + task bodies (same branch, reviewed).
- [x] Lessons bumped: verify-engine-guarantees-in-source (spike docs too),
      out-of-context-review-pass.
- [x] Goldens task deferred to backlog with an explicit entry gate.
