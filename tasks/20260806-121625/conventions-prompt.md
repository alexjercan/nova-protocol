# Handoff - extract the Rust CONVENTIONS.md

## Your job

Produce `tasks/20260806-121625/CONVENTIONS.md`: the Rust house style for
nova-protocol, **extracted from code the repo already writes well**, not
invented. Deliver it as a numbered list of candidate rules the owner accepts or
rejects one by one. It is promoted to the repo root once settled.

## The method the owner chose

> "we can do a short Q&A by checking old untouched files before using agents or
> tools on the repo"

and

> "I extract candidates, you rule" - you read the oldest untouched files,
> extract recurring patterns as a numbered list of candidate rules with real
> before/after snippets, and the owner accepts or rejects each in one pass.

So: **read first, propose second.** Do not open a linter, do not survey the
whole workspace, and do not start from what you think good Rust looks like. The
style already exists in these files; your job is to name it.

## Read first

| File | Why |
| --- | --- |
| `~/personal/scufris/CONVENTIONS.md` | the shape to copy. Python, but the format is the point |
| `~/AGENTS.md` | global style rules. Binding |
| `AGENTS.md` (repo root) | repo conventions, incl. the existing Code rules section |
| `tasks/20260806-121625/notes/07-comments-and-docs.md` | **the measurement. Read before proposing any comment rule** |
| `tasks/20260806-121625/NOTES.md` | the problem and the confirmed decisions |

## The shape to copy

`~/personal/scufris/CONVENTIONS.md` is short. One `##` section per rule. Each
section states the rule, shows a real code block, gives the rationale in one or
two sentences, and where relevant names the tool that would violate or enforce
it (that file ends by naming the exact ruff ruleset that would silently undo its
own convention - that kind of note is valuable, include the Rust equivalents:
clippy lints, rustfmt settings, `#![warn(missing_docs)]`).

It is binding and opinionated. It is not a tutorial and not a list of things
everyone already does.

## Source files - read these, in this order

Oldest untouched production files, by last-commit date. They predate the recent
churn, so they represent settled style rather than in-flight work.

| File | Date | Why it is a good source |
| --- | --- | --- |
| `crates/nova_gameplay/src/asset_ref.rs` | 2026-07-21 | model module doc: states the problem before the solution; hand-written `Clone`/`Debug` impls with the reason given |
| `crates/nova_gameplay/src/input/mod.rs` | 2026-07-21 | model orienting doc ("Touch this module when adding a new way to command a ship"); per-module prelude; plugin shape |
| `crates/nova_scenario/src/variables.rs` | 2026-07-21 | |
| `crates/nova_scenario/src/objects/binding_input.rs` | 2026-07-21 | |
| `crates/nova_os/src/command.rs` | 2026-07-28 | |
| `crates/nova_editor/src/ui/drawer.rs` | 2026-07-14 | |
| `crates/nova_editor/src/config.rs` | 2026-07-14 | |
| `crates/nova_ui/src/font.rs` | 2026-07-29 | |
| `crates/nova_assets/src/scenario/craft.rs` | 2026-07-18 | |

Add `crates/nova_editor/src/ui/card.rs` and `tooltip.rs` (2026-07-29) if you
need more UI-side evidence.

## Already settled - do not re-litigate

- **Delete the 91 `/// Glob-import surface: ...` boilerplate doc lines.**
  Owner ruled: the line says nothing the `pub mod prelude` declaration does not.
  The convention becomes: preludes get no prose, and CONVENTIONS.md states once,
  globally, what a prelude is and what it should contain. Generalize this into a
  rule about repeated boilerplate docs.
- **Do not propose a what-comment purge.** It was measured and rejected: 83% of
  sampled comments are why-comments, there is zero commented-out code, 3
  TODO markers workspace-wide, and a strict purge yields ~440 lines of 155,587.
  See `notes/07`.
- **The real comment problem is volume and staleness**, not noise. Rules should
  target: docs citing task artifacts (`"see this task's DECISION.md"`,
  `"DECISION fork 4"`, bare task ids at `nova_assets/src/portal/mod.rs:3`),
  recorded history, duplicated manuals (`nova_probe/src/lib.rs` carries 100
  comment lines in 168 duplicating `.claude/skills/probe/SKILL.md`), and
  multi-paragraph rationale essays. Proposed test: **a comment must survive the
  next refactor.**
- `#![warn(missing_docs)]` is already on all 16 crates. Any comment rule must be
  compatible with it - "delete the doc" is not available for public items, so
  the rule has to be about what the doc says.

## Areas the source files will likely yield rules about

Derive these from the files; do not assume they are all real.

- Module doc (`//!`) structure: what the module owns, when to touch it, what it
  deliberately does not own. Two of the source files do this well.
- `///` on public items: what and why, per `AGENTS.md`. When units, ranges and
  defaults belong in the doc (`flight/state.rs` is a good example of this done
  right, though it is not an old file).
- Prelude discipline: one per module, what goes in, what stays out. Note that
  `nova_ui`'s prelude is effectively dead - 81 in-src deep-path imports against
  3 prelude imports - so whatever the rule is, it is not currently enforced.
- Plugin and `SystemSet` shape: one plugin per subsystem, sets for ordering.
  `input/mod.rs` is the model; `nova_gameplay/src/plugin.rs:80-101` (13 loose
  leaf plugins) is the counter-example.
- Hand-written trait impls over derives when a derive would add a wrong bound -
  `asset_ref.rs` states the reason inline and is a good "comment why" exemplar.
- Where tests live. Both `#[cfg(test)]` inline and `src/*/tests/` are in use;
  several files are majority test code. Decide whether that is a rule or just
  a fact.
- Lint suppressions: 54 workspace-wide, 37 of them `clippy::type_complexity`.
  Worth a rule on whether a suppression needs a justifying comment.

## Rules for your output

1. **Every candidate rule cites a real file:line** as its source, and shows the
   actual snippet - not a paraphrase.
2. **Every rule needs a counter-example from this repo.** If nothing in the tree
   violates it, it is not a rule worth writing down; it is already universal.
3. **Say how many places violate it.** A rule with 80 violation sites is a
   refactor task; a rule with 2 is a cleanup. The owner needs that number to
   rule on it.
4. **Mark each rule enforceable or judgment.** Enforceable = a clippy lint,
   rustfmt setting or test can check it. Name the mechanism.
5. **KISS and YAGNI.** A rule nobody asked for and nothing violates does not go
   in. Aim for under 15 rules; fewer is better.
6. Do not propose rules about the four-crate split, the probe split or any other
   in-flight refactor. CONVENTIONS.md is about how code is written, not how it
   is arranged.

## Constraints

- ASCII punctuation only: `-`, `--`, `...`, `->`, straight quotes. No em dashes,
  smart quotes, typographic ellipses or arrows. This applies to the file you
  write.
- NixOS: any cargo command runs as `nix develop --command cargo ...`.
- Do not run `cargo test` or `cargo clippy` locally. CI owns both. `cargo fmt`
  and `cargo check` are fine.
- Do not edit any source file. This task produces one markdown file.
- Do not create tatr tasks. Do not start a branch or worktree.

## Done when

`tasks/20260806-121625/CONVENTIONS.md` exists as a numbered candidate list, each
rule with source citation, snippet, counter-example, violation count, and an
enforceable/judgment marker - ready for the owner to accept or reject in one
pass. Report anything the source files disagree with each other about; a
disagreement is a decision the owner has to make, not one for you to settle.
