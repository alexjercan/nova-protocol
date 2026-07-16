# Retro: Canonical enforcement - bare-ref lint (Option A)

- TASK: 20260717-002133
- OUTCOME: landed (squash 9f0fbbd0), review APPROVE round 1, targeted suites green.

## What was built

A static lint (content_lint + portal generator) that rejects a bare (scheme-less)
asset ref in content, making the self://+dep:// namespaced model canonical. Detection
is an asset-extension heuristic; the hard no-bare guarantee is structural (a bare
ref 404s at load now that base art lives under assets/base/).

## What went well

- The design constraint (why not a type-level ban) resolved cleanly once traced:
  the generic content walk can't type-distinguish AssetRef from any string, and
  AssetRef::deserialize can't be strict because the merge rewrite round-trips bare
  resolved paths through it. The extension heuristic + structural 404 backstop is
  an honest, correct answer - and documented as such, not oversold.
- Tested BOTH directions (catches bare refs; does NOT flag ids/names/messages incl.
  a slashed message), which is the real risk for a heuristic.

## What went wrong / difficulties

- **The full `cargo test -p nova_assets` build was KILLED twice** mid-compile
  (not OOM - 11Gi free, no competing procs at check time; likely transient
  resource pressure from parallel jobs building master's advancing commits). The
  full suite builds ~11 integration binaries, each linking all of bevy. Splitting
  into `--lib` + the 2-3 affected `--test <name>` targets built fine and covered
  exactly the changed code. Lesson: when a heavy full-suite build keeps getting
  killed, run the affected test TARGETS individually - smaller link steps, and it
  still covers the change.
- Two synthetic fixtures (content_lint_gate, portal_install) and one lint_walk
  fixture carried an incidental bare ref that the new gate flagged; made them
  canonical. Expected fallout of turning a convention into an error.

## What to improve next time

- For a new gate that turns a previously-legal pattern into an error, grep the
  TEST fixtures for that pattern up front (not just the shipped content) - fixtures
  use the old convention too and will trip the new gate.
