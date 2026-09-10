# Recovery record

The 2026-09-09 nightly review did not finish. This file records what was lost,
what was recovered, and from where. `TASK.md` holds the findings themselves.

## Why it stopped

The nightly briefing service died at 2026-09-09T23:51:03+03:00, 50m48s into a
run whose deadlines had more than seven hours left. A red-team review lane in
the SIBLING project (`personal/scufris2`) pointed a bounds-probe at a
`/dev/zero` symlink; an unbounded `Path.read_bytes()` grew that Python process
to about 29.2 GB RSS and 28.3 GiB of swap. The kernel killed it at 23:51:02.616.
Because it sat inside `scufris-briefing-nightly.service`, whose `OOMPolicy=stop`
and `KillMode=control-group`, systemd tore down the whole cgroup - the collector
and BOTH project sources with it. Nova's review was collateral, not a cause.

Diagnosis: Scufris job `582a68032ce3`. Its prioritized fixes are briefing
infrastructure and are deliberately untouched here.

## What the interruption cost

Nova's outer review session reached no `end_turn` and wrote no envelope. Its
last record is a successful Bash result at 20:50:28.354Z - a verification
command it never got to write down. Everything after that was lost:

- Adjudication of the G3+G4, G5a and G5b lane reports (recovered here).
- Dispatch of G6-G11 (never happened).
- The nightly answer the collector would have ingested.

The morning briefing's claim that G3-G11 "were not reached" is right about
DURABLE TASK CONTENT and wrong about work performed: seven lanes had already
finished.

## Sources

Outer Claude session `48668919-17fc-40b4-a724-08da813a449d`, under
`~/.claude/projects/-home-alex-personal-nova-protocol/`. All 16 dispatched
reviewer lanes reached `end_turn`; their transcripts sit in that session's
`subagents/` directory. Reviewed tree: `88a7445b4`, which is still `master`
today, so no finding below needed re-basing.

| Lane | Agent | Finished (UTC) | Group | Fate |
|-|-|-|-|-|
| correctness | `agent-a1bc030885be32442` | 20:16:52 | G1 | adjudicated 2026-09-09 |
| contracts | `agent-a83c51e85797e630d` | 20:13:07 | G1 | adjudicated 2026-09-09 |
| craft | `agent-aff0bbd8d1e75d222` | 20:14:07 | G1 | adjudicated 2026-09-09 |
| performance | `agent-a6e1fe85da7402db9` | 20:13:59 | G1 | adjudicated 2026-09-09 |
| feel | `agent-a89f926d46f757b66` | 20:13:29 | G1 | adjudicated 2026-09-09 |
| correctness | `agent-ae3c9c9c52c8396f8` | 20:33:06 | G2 | adjudicated 2026-09-09 |
| contracts | `agent-ae766a800af365893` | 20:31:49 | G2 | adjudicated 2026-09-09 |
| craft | `agent-a0066c1b5889ca8ed` | 20:25:48 | G2 | adjudicated 2026-09-09 |
| feel | `agent-a01460df182a029f0` | 20:32:54 | G2 | adjudicated 2026-09-09 |
| correctness | `agent-a318df9d833c645b0` | 20:50:09 | G3+G4 | RECOVERED 2026-09-10 |
| contracts | `agent-aba9b605f8637b620` | 20:47:41 | G3+G4 | RECOVERED 2026-09-10 |
| craft | `agent-a3b5f1a4d8de4d093` | 20:44:53 | G3+G4 | RECOVERED 2026-09-10 |
| correctness | `agent-aa9f18eec4385841a` | 20:47:33 | G5a | RECOVERED 2026-09-10 |
| craft | `agent-a7ba5208138c2fa8a` | 20:45:48 | G5a | RECOVERED 2026-09-10 |
| correctness | `agent-a9fa4529cdb067f7e` | 20:50:11 | G5b | RECOVERED 2026-09-10 |
| contracts | `agent-a1a5f1ee5786f579e` | 20:48:55 | G5b | RECOVERED 2026-09-10 |

Each recovered lane's dispatch prompt and full report is preserved verbatim
under `lanes/`. Those are RAW LANE CLAIMS, not accepted findings; `TASK.md`
records which survived verification.

## The dispatch differs from the plan

The Groups table in `TASK.md` is the plan the outer session wrote at 23:02.
What it actually dispatched differs, and the recovered findings follow the
dispatch:

- G3 and G4 were dispatched as ONE group over four commits: `45372be15`,
  `ba0cc418f`, `62043f3b5`, `9e69ac196`.
- G5 was split into two path-scoped halves. **G5a**: the three largest new loop
  producers of `7fdd25222` (`loop_hull_generate.rs`, `loop_helm_orders.rs`,
  `loop_goto_standoff.rs`). **G5b**: the rest of that commit's Rust and script
  surface outside those three.

## The outer session's unwritten verification

Its final two tool results survive and had already confirmed four lane claims
before it died. They are reproduced in the disposition below rather than
re-derived from scratch:

1. `loop_requested()` is presence-only where its two siblings parse `0`/`1`.
2. `producers()` walks `FIGURES` only.
3. Six `NOVA_*` names return 0 hits in `docs/environment-variables.md`.
4. `manifest.txt` carries `fresh` on rows `package_import` writes `frozen`,
   under a header pinning `a5da7efdb`.

## Recovery method

Read-only. No implementation file was edited, no check re-ran the review, and
no briefing infrastructure was touched. Every recovered claim was re-derived
against the worktree at `88a7445b4` by reading the code it names.
