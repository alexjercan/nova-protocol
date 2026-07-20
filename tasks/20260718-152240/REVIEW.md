# Review: content lint - merge balance audit into lint + per-mod located report

- TASK: 20260718-152240
- BRANCH: feature/content-lint-report

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings produced by a fresh reviewer with no sight of the implementing
session. In-session pass re-verified the load-bearing claims (the flagged stale
doc lines exist verbatim; file provenance attributes the ledger finding to
`ledger_ch4.content.ron`; shipped mods carry zero input-overlap findings) before
adopting. The branch delivers the goal: `audit` is gone from the CLI, `content
lint` runs reference/geometry + balance + input-overlap in one pass sharing one
located report, exit codes and ack scoping are preserved, and the flight-rig
drift guard pins the reserved list against the real rig. All findings are
non-blocking (stale doc mentions the audit->lint sweep missed); fixed anyway as
they are squarely in the task's doc-surface-sweep scope.

Check suite (out-of-context reviewer + re-run in-session): `content_report_gate`
2/2, `content_lint_gate` 2/2, `balance_audit_gate` 1/1, `content_report` lib
3/3, `flight_rig_reserves` 1/1; `cargo check -p nova_assets --all-targets`
clean; whole-tree lint clean at exit 0; md + html reports render with accurate
provenance.

- [x] R1.1 (MINOR) crates/nova_assets/src/balance.rs:36 - Module doc comment
  still reads "the `content` CLI's `audit` subcommand prints the full table";
  the subcommand no longer exists.
  - Response: fixed - repointed to "the `content` CLI's `lint` runs it in one
    pass with the reference checks (the balance audit was folded into `lint`)".
- [x] R1.2 (MINOR) crates/nova_assets/src/lib.rs:7 - Crate doc still says the
  crate backs "the `content` CLI (`gen`/`lint`/`audit`)".
  - Response: fixed - now "(`gen`/`lint`; the balance audit and input-overlap
    check are folded into `lint`)".
- [x] R1.3 (NIT) web/src/wiki/dev/development.md:154 - Prose says "One bin,
  three subcommands" but there are now two.
  - Response: fixed - "One bin, two subcommands".
- [ ] R1.4 (NIT) crates/nova_assets/src/lib.rs (file_of / build_report) -
  `file_of` matches an id against both scenario ids and section base ids,
  first-match-wins; a shared id across namespaces could misattribute a file.
  - Response: acknowledged, no change. Ids are practically disjoint today, the
    duplicate-id lint guards against collisions, and the `None` ->
    "(unknown file)" fallback is graceful. Left as best-effort by design; the
    `file_of` doc comment already notes the `None` case. Noted for a future
    author who introduces a shared namespace.
