# REVIEW - full game codebase state

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: review, backlog

## User facts

- Review the current game code at baseline `b7a56f0586` as a repository-state assessment.
- Scout the repository first, then build a review coverage graph.
- Review the codebase slowly in repeated batches of at most two concurrent subagents.
- Each reviewer must state exact checked and unchecked coverage.
- Add ECS, content, and proof review only where scouting shows that they fit.
- Review only. Do not fix code.

## Delivery

- Store the coverage plan in `GRAPH.md` in this task directory.
- Store adjudicated batch reports as `REVIEW-<slug>.md` in this task directory.
- Cover runtime, ECS, content, tools, UI, platform, and proof surfaces based on repository evidence.
- Revisit uncovered or weakly sampled code with more reviewer pairs.

## Verification

- Every report gives file and subsystem coverage, findings with `file:line` evidence, and explicit gaps.
- The final coverage audit maps every in-scope crate and major non-crate surface to a completed review or a named exclusion.
- No source, content, generated file, test, or documentation fix is made.

## Done when

- Scouting and `GRAPH.md` identify coherent review slices and dependencies.
- Review batches have covered the graph with no more than two active subagents at any time.
- Findings are adjudicated and duplicated claims are merged.
- Remaining limits and unverified claims are explicit.

## Completion

- Six scouts produced the dependency and review graph in `GRAPH.md`.
- Twenty-five paired review batches covered all mapped code-bearing surfaces without more than two active subagents.
- ECS, content, and proof specialists adjudicated seven cross-boundary claims.
- Batch evidence is stored in `REVIEW-01-*.md` through `REVIEW-25-*.md`.
- The consolidated verdict is `REVIEW-full-game-codebase-state.md`: 0 BLOCKER, 12 MAJOR, 42 MINOR.
- No source or content fix was made.
