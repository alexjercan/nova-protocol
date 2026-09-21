# Review 17: Probe capture and evaluation

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Specialist: proof trust-boundary audit
- Verdict: minor finding

## Finding

### MINOR - `examples/systems/README.md:90-100` - The no-shared-module rule contradicts current proof fixtures

The README says system ranges do not share modules and permits only a sibling module owned by one range. The repository has `examples/systems/shared/{editor_stage,editor_walk,section_aim}.rs`, reused by four editor-related ranges specifically to prevent duplicated literals from drifting.

A contributor following the current guidance would recreate the problem the shared fixtures were added to solve. This affects proof-authoring guidance only, not current verdicts.

## Specialist verification

The proof specialist independently confirmed:

- capability declaration, arming, and artifact presence remain distinct and fail loudly;
- malformed and partial artifacts cannot silently pass;
- in-process deadlines precede supervisor timeouts;
- timed-out runs still receive reports and checks;
- output cleanup is bounded to owned artifact names;
- the profile sandbox has a real cross-process fail-first proof wired into CI;
- the system catalog has 65 examples and 419 outcome slugs with no roster drift.

No trust-boundary finding survived.

## Coverage

Fully read every source and test file in `nova_probe`, `nova_probe_cli`, and `nova_perf_web`. Checked crate manifests, root environment-contract tests, proof documentation, CI probe wiring, and the complete static example catalog/outcome roster. Individual system example bodies were sampled and catalog-checked rather than read end to end; deeper example review remains in batch 19.

Not checked: no Cargo command, probe run, game/GPU process, workspace test, or Clippy run. Fixture artifact bytes were not all inspected manually. Timeout behavior was verified from source and tests, not by hanging a process.
