# Owner decisions

Every ruling from the understanding-phase Q&A. Binding. Where the owner's words
carry nuance, they are quoted verbatim.

## What "better" means

Ranked success criteria:

1. **Cold-read navigability**, measured by a benchmark suite (see
   `../benchmark/README.md`)
2. **Deletion count**
3. **Reduced coupling** - expected to fall out of 1 and 2, not independently
   gated

> "I don't want this to be just shuffling code around but still getting to an
> actually good result"

> "It's more about readability and being able to go through the code structure
> fast and being able to tell what each module/system does from the folder
> structure."

## Deletion targets - all three

1. **Stale narrative and prose volume.** Task-artifact references, recorded
   history, duplicated manuals, multi-paragraph rationale essays. Includes the
   91 `/// Glob-import surface: ...` boilerplate lines.
2. **Duplicated implementations.** Scroll clamps, the list+details pattern,
   the twin orbit-camera scenes, keybind chips.
3. **Dead and lying surface.** Pub items nothing reads, `render: bool`, dead
   feature flags, the `nova_debug` feature leak.

**Rejected: a literal what-comment purge.** Measured at ~440 lines of 155,587.
The premise was tested and did not hold. See `07-comments-and-docs.md`.

## nova_gameplay

**All four seams.** Layer the crates rather than adding a shared-types crate.

Order: **CORE <- FLIGHT <- HUD <- NOVAOS**. Verified acyclic; see
`03-nova-gameplay.md` for the back-edge evidence.

Rejected: a `nova_gameplay_types` crate for shared markers. The graph is already
layerable, so the crate is unnecessary. YAGNI.

## nova_probe

Owner's words:

> "we do the split into two crates and move the bin to `nova_probe_cli` it's
> basically a mix of all three options"

So, all of:

- Split into `nova_probe` (in-game collection library) and `nova_probe_cli`
  (host harness). The process boundary becomes a crate boundary.
- Rename modules to `capabilities/`, `evaluation/`, `report/` so the tree states
  the pipeline.
- Evict `fixtures.rs`, `profile_sandbox.rs`, `bin/perf_web.rs`.

Rejected, on evidence: a single `NovaProbePlugin` spanning collect -> evaluate
-> report (impossible across the process boundary), and a `trait Capability`
covering configuration (the three configs do not unify).

## nova_events

**Correctly used. Do not migrate.** The owner's own reading was right:

> "nova_events is for modding purposes it exposes certain events to be used from
> RON files; when I use observers for things like 'add renderer on a section
> once it gets spawned' that is a code game logic functionality not a modding
> event"

Confirmed by `crates/nova_events/src/lib.rs:1-9` and by usage (nova_scenario 50
refs, nova_gameplay 10, and those 10 are exactly the scenario-observable
moments).

What is wrong is the AGENTS.md line "Cross-subsystem communication through
`nova_events`, not direct coupling". It reads as a general architecture mandate
and misled an audit agent into flagging 46 healthy files. **Reword it.**

## AGENTS.md

**Fix first, as step one.** Before any refactor task starts. Every subsequent
agent reads the corrected version.

Rejected: rewriting it at the end.

## Comments and conventions

- **Delete all 91 `/// Glob-import surface: ...` lines.** The line says nothing
  the `pub mod prelude` declaration does not.
- CONVENTIONS.md derivation: **owner-ruled candidates.** The agent extracts
  recurring patterns from old untouched files with before/after snippets; the
  owner accepts or rejects each in one pass. Derived from old files *before*
  agents or tools sweep the repo.
- Model: `~/personal/scufris/CONVENTIONS.md`.

## Benchmark

Six personas: `blind`, `docs`, `tree`, `rustdoc`, `owner`, `modder`. A seventh,
`human`, was added later as an optional second control answering the same
papers as `owner`.

Measurements, all of: tool calls to correct answer (primary), graded correctness
(right / partial / wrong / gave-up), wrong-path detours, self-reported
confidence, **plus** a design tier:

> "We also give them a coding task and they create a markdown file with
> basically the understand step so they provide a 'NOTES.md' with what they
> understand and what the change they would make and then we use a reviewer
> agent to grade them"

Tier 2 tasks selected: **new ship section type**, **new NOVA OS app**, **new
scenario action + event**. (Owner declined "add a new probe check".)

Modder persona: build an actual mod, wiki + `webmods/` examples only.
**Objective verdict** - the mod must pass `content -- lint` and load. Owner's
sandboxing instinct:

> "question is how to guardrail it to only check these files? maybe copy paste
> them into a /tmp folder and moving the agent there somehow"

First answer: sandbox in `/tmp` plus a post-hoc transcript audit. The guardrail
was soft; contamination was made *detectable*, not impossible.

**Superseded 2026-08-07.** Owner rejected the soft guardrail:

> "I don't really like that we need `audit-transcript.sh` - this is the problem
> statement of this part"

Each persona now runs in its own Docker image holding only that channel. The
repo is never mounted, `blind`'s `.md` files are deleted rather than
off-limits, and rustdoc's `src/` pages are stripped so that persona cannot
collapse into `blind`. The audit script is deleted; there is nothing left to
grep for. Residual hole: the network stays up for the API, flagged (not
blocked) by `aggregate.py`.

Same instruction reshaped the rest of the harness: papers are generated per
persona from the key, tier 2 is one run per task, results aggregate to
JSON/CSV, and the report is generated HTML - explicitly modelled on how
`nova_probe` collects artifacts.

Answer key: **drafted with citations, owner ratifies.** The drafter also
designed the refactor, so every expected answer carries a `file:line` citation
to make that bias auditable.

Baseline timing: **after the AGENTS.md fix**, so the delta is attributable to
structure rather than to a stale table.

Owner runs a fixed 8-question Tier 1 subset plus one Tier 2 task, not the full
suite.

## Out of scope

- **Tests.** > "tests as you said should be a separate task, I will see to it
  **do not create it**."
- Performance. No runtime metric is a success criterion.
- Any move that does not reduce the cost of finding or reading something.

## Settled since

- **`TREE.txt` line counts: NO** (owner, 2026-08-07). Names only. Counts would
  be a second information channel and would flatter the baseline, shrinking the
  delta the persona exists to measure. The flag survives in `sandbox.sh` only so
  the ruling is visible and reversible.
- **`TREE.txt` source: git, not `find`** (owner, 2026-08-07). `git ls-files
  --cached --others --exclude-standard` minus `tasks/` - what a fresh clone
  shows. The `find` version listed build output and editor droppings and needed
  a hand-maintained prune list to stay honest. 3,257 lines -> 949.
- **Auth in the container: OAuth, no API key.** `claude auth login` credentials
  are copied in and work. Verified end to end.
- **Two benchmark runs, owner-driven** (owner, 2026-08-07). The baseline and one
  final run at the end of the epic. **Not per seam** - the earlier plan had L9
  rerunning it four times. **The owner starts and runs both**; no lane and no
  agent runs the benchmark. A lane that needs a number stops and prompts.
- **Re-key once, before the final run** (owner, 2026-08-07). The epic moves the
  things `keys/tier1.json` cites, so the final run needs an updated key.
  Question text frozen; only `expect` and `citation` change; a question whose
  answer no longer exists is a finding, not a re-key. Comparability of the two
  keys was raised and dismissed: > "I don't care about that it's fine."

- **Benchmark lives at `<root>/benchmark/`** (owner, 2026-08-07). It is a
  rerunnable tool that outlives the epic. `sandbox.sh:38-42` gets a named
  exclusion list covering `tasks/` **and** `benchmark/` - one chokepoint, so it
  covers the tar copy and `TREE.txt` together. Lands in L0, before the
  baseline. Not done until `sandbox.sh build tree` shows no `benchmark/` path.
- **F61 - epsilon compare in `Equal`** (owner, 2026-08-07). Not a second
  `ApproxEqual` node, not documented-as-is.
- **F47 - make the headless mode real** (owner, 2026-08-07). Gate hanabi,
  skybox, post and the HUD on `render`. Deleting the field was rejected.
- **F66 - a no-lock torpedo is a MISFIRE, and stays one** (owner, 2026-08-07).
  Behavior unchanged; one comment so the next reviewer does not re-report it.

## Open

- **The L7 escape hatch.** If owner review of the question set drags, F17 and
  F28 can land in place during L4's window rather than waiting for the
  extraction. Decidable when the ratification timeline is known.
- **F84** (`proc-macro-error2` future-incompatibility) needs its own tatr task.
  Not this epic.
