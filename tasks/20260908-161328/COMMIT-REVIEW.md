# Reviewed commits and four-page restyle

The user requested commits for the reviewed work and adaptation of the original
four opening pages to the accepted style. Work stays on master and in this task.

## Initial commit scope

1. Shared illustration sources, palette, two ship models, frontal character
   drawings, deterministic exporters, tests, and the three lore proposal SVGs.
2. Approved place names, ship and crew identities, public lore links and
   registration, redirects, publication rules, shared season outline, and only
   the existing lore changelog entry.
3. The authorized story task, original four-page draft, accepted three-shot
   study, decisions, and review evidence.

The pending diff was reviewed before staging. Elena's image alt text still
said three-quarter view; it was corrected to match the accepted frontal image.
No wiki files, game code or assets, dependency files, other tasks, or unrelated
changelog entries belong to these commits. Stage explicit paths and commit in
the same operation; verify the index is empty before each operation.

## Checks before committing

- Full website CI: formatting, lint, all website tests, and root build.
- Thirteen illustration unit tests.
- Deterministic checks for the shared lore exports, accepted three-shot study,
  original four-page draft, and five original public crew portraits.
- Earlier current-revision browser evidence covers the accepted study and the
  built lore articles at desktop/mobile sizes. The four-page restyle needs new
  rendered inspection and evidence before its own commit.

No full Rust workspace checks are relevant to these Python, web, and story
changes. No commit implies deployment or publication of a private draft.

## Completed reviewed-work commits

- `ca1ccaef6`: Add shared comic illustrations and lore design exports.
- `8ca7c24f3`: Establish the named Saturn cast and shared opening story.
- `7e8095fd8`: Preserve the opening comic and accepted visual studies.

The four-page adaptation is a separate follow-up commit. Its current source,
rendered evidence, preservation checks, and review are in
[RESTYLE-REVIEW.md](RESTYLE-REVIEW.md). The later `.scufris.toml` commits
`675ce45c6` and `489aeeb35` are concurrent work and are preserved. Nothing was pushed.

## Four-page restyle scope

Preserve a byte-for-byte comparison copy of the existing pages and generator.
Keep the story beats, dialogue, orientation-card facts, and departure endpoint.
Use the accepted frontal faces, close framing, necks and linework, broader
palette, and shared industrial ships. Keep required scene props local. Do not
add the rescue, invent dates or specifications, or register the comic publicly.

## Card approval and season-writing task

The user accepted the card placement and requested a commit. This follow-up
commits the card-only page change, its updated checks and evidence, and the
refocused task. Task `20260908-161328` is now "Write the shared season-one story";
it stays OPEN at priority 50 with `v0.13.0,story,comic` tags.

The writing plan covers the complete causal outline, scene sequence, comic
scene action/dialogue, matching campaign story brief, and continuity review.
The four-page draft and accepted art are groundwork, not the task's final goal.
Expression variants are recorded as later illustration work; none were drawn.
No season scenes were written or new plot choices made in this task update.

The production outline only corrects its card-placement note and updates its
active task pointer. It remains unregistered. No public article, wiki, gameplay,
shared illustration, changelog, other task, or publication input changes.

Pre-commit proof is kept in `proof/first-panel-card/`: deterministic generation
checks, the card-only XML comparison, current browser checks, Markdown rendering
and links, and full website CI. The last includes the site build, not Rust or
campaign-runtime validation. Earlier evidence stays intact. Nothing is pushed.

Reports: [generation](proof/first-panel-card/pre-commit-generation.txt),
[15 illustration tests](proof/first-panel-card/pre-commit-unit-tests.txt),
[website CI](proof/first-panel-card/pre-commit-web-ci.txt),
[Markdown](proof/first-panel-card/pre-commit-markdown.txt), and
[commit checks and explicit scope](proof/first-panel-card/commit-checks.json).
Another task appeared concurrently; its paths are recorded as excluded, not
checked or staged as part of this work.
