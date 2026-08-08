# Adopt flow v2: root LESSONS.md, clean tatr check, AGENTS.md flow section

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: chore, process

## Story

As a repo in the flow ecosystem, I want the v2 /flow conventions in place -
root LESSONS.md ledger, clean tatr check, AGENTS.md pointing at /flow - so
development here compounds the same way as everywhere else. Part of the
six-repo adoption goal (umbrella: nix.dotfiles tasks/20260720-171807).

## Steps

- [x] Ledger at the root: move docs/LESSONS.md to LESSONS.md (git mv) - or
      create it from the lessons-skill format if the repo has none - then
      run the doc-surface sweep for every reference to the old path
      (AGENTS.md, README, scripts, CI guards, wiki pages) and update them.
      Bring the ledger to format: bare counts until promotion, a
      "## Pending promotions (3+ occurrences, user decides)" section;
      move unpromoted (x3)+ entries there; keep existing PROMOTED/absorbed
      annotations as they are.
- [x] Fix tatr check findings best-effort, assuming recorded work was done
      properly where the record supports it:
      - closed-unchecked: tick Steps boxes whose close-out notes or landed
        commits evidence the work shipped; genuinely unshipped steps stay
        unticked and go on the residue list;
      - closed-not-approved: normalize nonstandard-but-approving verdict
        lines (e.g. "Verdict: APPROVE", "**APPROVE**") to
        "- VERDICT: APPROVE"; a review that really ended unapproved goes on
        the residue list untouched;
      - bad-severity: map to the nearest of BLOCKER/MAJOR/MINOR/NIT
        (LOW -> MINOR, NOTE/INFO/OBSERVATION -> NIT, FIXED -> the severity
        it had, keeping any "fixed in-review" note in the text), recording
        the mapping in the close-out.
- [x] AGENTS.md: add or refresh a "Development flow" section stating: /flow
      drives development here (plan/work/review/compound via tatr tasks,
      sprout worktrees, out-of-context round-1 reviews, DoD proofs with
      test:/cmd:/manual: notation); LESSONS.md at the repo root is the
      lessons ledger, read before starting any task; `tatr check` (plus
      `--ledger LESSONS.md`) is the conformance gate. Keep the section
      short; do not restructure the rest of the file.
- [x] Verify: tatr check exit 0 (or residue listed in the close-out),
      tatr check --ledger LESSONS.md exit 0, and the repo's own check
      suite still green.

## Definition of Done

- LESSONS.md at the repo root, old docs/ path gone, no stale references
  (cmd: test -f LESSONS.md && test ! -f docs/LESSONS.md && ! grep -rn "docs/LESSONS" --include="*.md" --include="*.sh" .)
- tatr check clean or residue documented (cmd: /home/alex/personal/tatr/tatr check;
  manual: user reviews the residue list at the goal's Finish)
- ledger lints clean (cmd: /home/alex/personal/tatr/tatr check --ledger LESSONS.md)
- AGENTS.md names /flow and LESSONS.md (cmd: grep -n "flow\|LESSONS.md" AGENTS.md)

## Notes

- Use the tatr binary at /home/alex/personal/tatr/tatr (the installed one
  may predate the check subcommand).
- Preserve history honestly: normalizations keep meaning; ticks record
  verifiably shipped work only (linter-adoption cleanup, per the precedent
  in tatr's own 20260720-152503).

## Close record

Shipped on branch chore/flow-v2-adoption (worktree, not landed).

What changed:

- Ledger: git mv of the ledger from docs/ to the repo root (LESSONS.md).
  Reference sweep updated AGENTS.md, Trunk.toml, nova_meta_gen doc comments
  (lib.rs + main.rs), web/src/wiki/dev/development.md and
  keeping-docs-in-sync.md, docs/README.md, scripts/wipe-docs.sh,
  scripts/check-docs-clean.sh, .github/workflows/release.yaml, and 65
  occurrences across 56 historical files, 52 distinct tasks (mechanical path update).
  The ONLY remaining old-path strings in the repo live in this TASK.md
  (its Steps/DoD text, which describe the move itself) - the DoD grep is
  self-referential there; excluding this file it returns zero matches.
  docs/ now wipes to only its README.md; the wipe script's repo-root sanity
  guard now checks root LESSONS.md + docs/README.md.
- Ledger format: all unpromoted (x3)+ lessons moved into "## Pending
  promotions (3+ occurrences, user decides)": render-output-eyeball (x5),
  authored-vs-derived-values (x4), advertised-but-unwired (x3), plus the two
  entries previously duplicated with a nonstandard (x3, PENDING) annotation
  (prose-from-diff-not-intent, verify-stale-brief-against-tree - now single
  entries in Pending with their ids), plus out-of-context-review-pass
  (positive, x31) which the ledger lint exempts only because "positive,"
  annotates its count - moved per this task's every-unpromoted-x3+ rule with
  a note that it is already /flow's round-1 practice. PROMOTED/absorbed/
  "enforced in AGENTS.md" annotated entries stayed in place, annotations
  intact. tatr check --ledger LESSONS.md reports zero ledger findings.
- AGENTS.md: new "Development flow" section (/flow drives development; tatr
  tasks, sprout worktrees, out-of-context round-1 reviews, DoD proofs
  test:/cmd:/manual:; root LESSONS.md read before any task; tatr check +
  --ledger as the conformance gate); the LESSONS section and the
  ephemeral-docs paragraph repointed at the root path.
- CHANGELOG.md: one Internals & Tooling line for the move + record cleanup.

tatr check cleanup (89 findings at start: 33 closed-unchecked,
30 closed-not-approved, 26 bad-severity; 30 residue findings left, all
closed-unchecked):

- bad-severity 26/26 fixed. Scheme: severity token in the parens reduced to
  the vocabulary, qualifier kept verbatim as a bracketed note after the
  parens. Mappings: "SEV, fixed" -> (SEV) [fixed in-review] (x5: BLOCKER x2,
  MAJOR x2, NIT x1); qualifier moves for "agent" (x4), "user playtest",
  "recorded" (x2), "informational - accepted as intentional", "pre-existing"
  variants (x4), "left as-is", "visually confirmed", "ACCEPTED-AS-IS",
  "WON'T-FIX", "out-of-scope, deferred"; INFO -> NIT; VISUAL -> NIT [visual]
  (was listed under non-blocking NITs); "verification, not a defect" ->
  NIT [verification, not a defect]; SCOPE -> MINOR [scope, user feedback]
  (a user redirect executed in-round).
- closed-not-approved 30/30 fixed, no verdict invented - every REVIEW.md
  already ended approving in nonstandard form: bold **APPROVE** (2),
  missing "- " prefix (11, incl. 20260719-112011's user-adjudicated round-2
  APPROVE), "## Verdict: APPROVE" headings split into heading + standard
  line (4), "### Verdict:" round headings converted (1 file, both rounds),
  bare "APPROVE - land ..." lines under a "## Verdict" heading prefixed
  (10), one prose "Verdict: **APPROVE**." converted (20260716-124722). In
  20260713-222025/-225324 rounds are ordered newest-first, so the
  superseded round-1 line was relabeled "- Round 1 VERDICT (superseded by
  the round 2 APPROVE above): REQUEST_CHANGES" so the parser's last-wins
  rule reads the real outcome.
- closed-unchecked: 47 of 158 boxes ticked across 15 task files (see review R1.1), each verified
  per-clause against close-outs, REVIEW/RETRO/NOTES and landed commit
  diffs (three parallel reviewers, diffs verified as pure checkbox flips,
  evidence spot-checked). 110 boxes across 30 tasks stay unticked =
  the residue below.

Residue list (all closed-unchecked; counts = unticked boxes):

- 20260707-095020 (2): in-code PROMOTE() markers never added (promotions
  shipped directly); the docs/ catalog file never existed (record lives in
  the spike 20260708-110317).
- 20260708-200001 (1): full check-suite step - fmt/clippy unevidenced; its
  own RETRO calls the cargo test run root-package-only "false comfort".
- 20260708-224303 (5): closed wontdo, superseded by the audio refactor;
  no rig ever built under this task.
- 20260709-140559 (2): decision was keep-as-is, so no knob/exemption code
  and no test were ever built.
- 20260709-140620 (1): step self-labelled "Deferred, needs a user decision"
  (punted to 20260709-095043).
- 20260710-234115 (1): by-eye tint pass - "deliberately unticked... needs a
  human playtest".
- 20260711-180506 (1): visual playtest "NOT DONE - needs a human at the
  controls".
- 20260712-203345 (2): framing generalization shipped via a different
  mechanism than the step prescribes; the 12_hud_range beacon/torpedo
  verify clause explicitly not done.
- 20260712-212742 (1): tests step - two of its four named tests never
  shipped as specified.
- 20260712-215402 (8), 20260712-215957 (9), 20260712-215958 (7),
  20260712-223034 (6), 20260712-223035 (11), 20260712-223036 (7),
  20260712-223345 (8), 20260712-231141 (7): the superseded/wontdo lock
  family - closed by the deliberate-radar-locking spike takeover
  (20260713-082207); their own closures record "no code shipped" /
  "never started". 63 boxes.
- 20260713-220512 (1): minimal-rig reproduce step never done (mechanism
  found analytically; documented-as-designed).
- 20260714-214118 (4): OUTLINE/AMMO semantic consts never added; chrome
  pass descoped from ~12 files to 4; combat reds stayed local; the
  test + BCS_SHOT verification explicitly skipped.
- 20260715-204358 (1): verify step's search/see-also/deploy-subpath
  clauses unevidenced.
- 20260715-205825 (1): verify step's search/see-also + deploy-subpath
  clauses unevidenced.
- 20260715-210609 (1): verify step's deploy-subpath clause unevidenced;
  index links reasoned, not observed.
- 20260715-224013 (1): image annotations explicitly DEFERRED to follow-up
  20260715-231500.
- 20260716-231341 (6): closed as unnecessary ("we already ship base as an
  art pack"); nothing implemented.
- 20260717-003613 (1): tint legibility on the real gltf ship - NOT DONE,
  needs a human playtest (the spike's open question).
- 20260717-133332 (2): editor never actually run (bug falsified by source
  analysis); the conditional fix step's condition was false.
- 20260717-221106 (6), 20260717-221110 (5): "Closed without implementing"
  descope notes (direction changed to the cutting-only tool).
- 20260718-152225 (1): checker step as written (Python, allowlist just
  LESSONS.md) contradicted by what shipped (bash, README kept).
- 20260718-181305 (1): manual in-game Broadside check - "session is
  headless, so it was NOT performed".

Check suite results:

- tatr check: 30 findings, all the closed-unchecked residue above (was 89).
- tatr check --ledger LESSONS.md: zero ledger findings (the promotion-
  stalled trio is resolved); the command's process exit is nonzero only
  because the 30 task-residue findings are included in the same run.
- cargo check --workspace: green (1m39s cold; only the pre-existing
  proc-macro-error2 future-incompat note).
- cd web && npm run ci: green (prettier check, eslint, webpack build -
  wiki pages compile with the edited markdown).
- scripts/check-docs-clean.sh: "docs/ is clean (only README.md)";
  scripts/wipe-docs.sh: idempotent no-op confirmed.
- Full cargo test/clippy NOT run locally, per AGENTS.md (CI owns them);
  no code fixture references the moved path (grep include_str + literal
  "docs" over crates/tests/examples came back clean), so the move is
  invisible to the test suite.


## Review round 1 amendments (2026-07-20, applied post-land)

Per R1.1 the 20260708-194524 full-check-suite box (ticked on inference; no
recorded cargo test run) is unticked and joins the residue: 47/158 ticked,
111 boxes across 31 tasks. Counts corrected per R1.2. Note: these
amendments were claimed in REVIEW.md at landing but a scripted edit had
silently aborted before applying them (multi-line step text failed a
single-line assert); this commit is the repair, made immediately after the
land when the residue count exposed the gap.
