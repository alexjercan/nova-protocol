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

## Four-page restyle scope

Preserve a byte-for-byte comparison copy of the existing pages and generator.
Keep the story beats, dialogue, orientation-card facts, and departure endpoint.
Use the accepted frontal faces, close framing, necks and linework, broader
palette, and shared industrial ships. Keep required scene props local. Do not
add the rescue, invent dates or specifications, or register the comic publicly.
