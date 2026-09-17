---
name: nova-implement
description: Deliver Nova changes through an approved, code-backed design.
---

# Nova Implement

Use this skill by default. A decision, not an action, is the unit of work.
Follow [AGENTS.md](../../../AGENTS.md). Read code before proposing changes.

## Ground the change

1. State the intended behavior, what dies, and what may break.
2. Map the feature slice: owner, entry point, data flow, ordering, runtime IDs,
   content, UI, docs, callers, and closest proof.
3. Show exact paths and lines plus the relevant existing types and functions.
4. Show proposed types, fields, functions, and signatures.
5. Show a compact caller/callee graph before and after the change.
6. Name the behavior, failure, or invariant that verification will prove.

For a bug, reproduce it first and preserve the failure. For uncertain mechanics,
propose a disposable fixture or spike. Do not turn uncertainty into abstraction,
compatibility, optional fields, or fallback behavior.

## Get the decision

Treat interfaces, names, defaults, precedence, ownership, error behavior, and
new tests as decisions. Show options, consequences, and a recommendation.
Wait before adding a type, function, or test.

## Deliver the approved design

1. Change the owning interface first.
2. Use compiler errors and content failures to find consumers.
3. Update every caller, runtime ID, owned content file, UI, and invalidated doc.
4. Delete the replaced path, adapter, alias, default, comment, and stale test.
5. Run the cheapest proof that observes the real behavior.
6. Inspect assertions, reports, logs, audits, generated output, and frames.

Add permanent proof only for named stable behavior or a reproduced failure.
Use a unit test for pure logic, `nova-probe` for deterministic game behavior,
and `nova-bench` for player flows. Headless output cannot prove appearance.

