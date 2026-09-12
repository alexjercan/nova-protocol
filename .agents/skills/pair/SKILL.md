---
name: pair
description: >-
  Default Nova mode: code-backed proposals, stopping at user decisions.
---

# Pair

Use this mode unless the user requests another. Follow the always-on rules in
[AGENTS.md](../../../AGENTS.md). A decision, not an action, is the unit of
the loop.

## One decision at a time

1. Read the affected code. Run the cheapest relevant read-only check.
2. For a bug report or change proposal, show `Claim`, `Evidence`, `Change`,
   `Blast radius`, and `Verification` as required by AGENTS.md. Use small code
   excerpts and proposed signatures, not paragraphs of abstract terms. For
   later updates, show only what changed; do not repeat the whole proposal.
3. If the user's answer can change the outcome, show a short options table:
   option, consequence, recommendation. Treat interface, naming, default, and
   precedence choices as decisions. End with one concrete question.
4. Otherwise continue through mechanical work and approved plans without
   asking for confirmation. Do not stop only to ask whether to continue.

Do not change files while answering a question. Do not create task records
unless the user requests tracked work. For feature, bug, or behavior work,
read [Verify](../verify/SKILL.md) before choosing a reproduction or check.

## At a stop

- `Delta`: what changed, or the grounded proposal if nothing changed.
- `Verified`: the checks run, their results and evidence paths, plus limits.
- `Next`: the next action or the decision, options, and recommendation.

Put a decision's question on a separate final line. Do not add a question when
there is no decision left. Full workspace tests and Clippy remain opt-in;
Pair is not permission to run them before every edit or commit.
