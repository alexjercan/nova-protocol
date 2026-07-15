# Retro: dev wiki IA refactor + Extend-the-game guides

- TASK: 20260715-204358
- OUTCOME: shipped (landed 765cef3f); review APPROVE, 2 NITs fixed.

## What went well

- Spiked the direction first (grounded code sweeps of the three extension
  domains), got user buy-in on the IA and scope via AskUserQuestion, THEN built.
  No wasted content - the structure was agreed before writing.
- Guide content fanned out to five parallel subagents, each given a code-extension
  map as a STARTING point but told to verify every file:line and RON against the
  tree. They caught real drift in the maps (examples at repo root not per-crate;
  only 3 resistance rows needed thanks to the `(_, Kinetic)` wildcard; the plugin
  tuple line). Map-as-hint + verify-against-source beat map-as-truth.
- The out-of-context fact-check pass is what makes "the guide is accurate" a fact
  rather than a hope - it independently re-derived paths, enum sets, RON shapes,
  the portal CLI, and overlay semantics, and confirmed the two author-flagged
  snippets. It found only 2 NITs, both real.

## What went wrong

- Nothing structural. The two NITs (a missing `Stop` flight verb, a "two test
  arrays" overcount) were small factual slips a single generating agent made and
  the reviewer caught - exactly the split the process is designed for.

## Lessons

- `map-as-hint-verify-at-source` (positive): when delegating doc/codegen from a
  prior exploration map, pass the map as a starting hint but require the agent to
  re-verify each anchor against the live tree - the map carries the earlier
  session's line-number drift and simplifications, and the verify step both fixes
  them and deepens the output. Pair with an independent fact-check pass for
  generated how-to content. 20260715-204358.
