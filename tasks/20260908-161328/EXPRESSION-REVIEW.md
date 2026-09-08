# Reusable facial expressions: page-1 trial

## Request and scope

The user requested reusable expressions in the illustration scripts and a trial.
This follows the proposed page-1 Rina/Jonah comparison. It does not replace the
season-writing goal, add scenes, or authorize publication. After reviewing the
trial, the user requested its commit.

Comparison baseline: `b716d27b0618b914885cf512bc6bfb2b7463b6f2`, the accepted
first-exterior Baikal card revision. The later `.scufris.toml` commit is unrelated
and remains untouched. Other-task work is excluded, not reviewed or changed.

## Delta

- `scripts/nova_illustration/expressions.py` owns the original and new feature
  layers for Rina and Jonah. Their nose geometry and feature materials remain
  shared; brows, eyes, mouths, and nearby acting lines vary by character.
- `frontal_head` and all body helpers accept an expression name. `original` is
  the retained drawing and remains the default. `expression_names` reports
  only authored variants; unknown character/expression pairs are errors.
- The page-1 coffee exchange selects Rina `amused` and Jonah `wry`. Other
  appearances retain their original features. No generic face deformation,
  overpainted old mouth, new head angle, or new body staging is introduced.
- The private [comparison sheets](expression-study/README.md) show the same
  sources in original/variant pairs and comic/lore colors. Labels stay outside
  the color filter. These layouts do not own duplicate feature geometry.
- Elena, Leila, and Tomas have no new expressions in this trial. Samir remains
  outside the shared head registry. This is not a complete emotion library.

## Render review

Inspected the [current page 1](proof/expressions/page-01.png),
[comic comparison](proof/expressions/expressions-comic.png),
[lore comparison](proof/expressions/expressions-lore.png), and
[390-pixel board view](proof/expressions/390-page-1.png).

The first render made Rina's eyes too narrow. The revised eyes stay more open.
Jonah's curved lower-lip crease produced an unwanted filled crescent; an
explicit straight crease removes that extra dark shape. The final comparison
keeps visible necks, unchanged head contours and hair, and the original body
poses. The smiles remain small enough for the quiet exchange.

The phone board is still a landscape overview. Full-size SVG zoom and text
transcripts remain necessary; this trial does not solve mobile comic reading.

## Verified

- [20 illustration tests](proof/expressions/unit-tests.txt): default selection,
  feature replacement, retained likenesses and body poses, strict names,
  immutable variants, repeated instances, and color-only presentation pass.
- [Five generation checks](proof/expressions/generation.txt): opening board,
  expression sheets, accepted three-shot study, crew portraits, and public
  design exports are deterministic and current.
- [Source and scope check](proof/expressions/source-and-scope.json): all five
  default heads and body helpers match the comparison revision byte-for-byte.
  Replacing just the two new head feature layers reconstructs the old page 1
  exactly. The board changes only its embedded page-1 SVG; pages 2-4 are exact.
- The same source check protects all 20 public lore assets, public sources and
  exporters, ship/scenery/palette sources, accepted studies, and earlier proof.
  Six SVGs parse and have unique, resolved IDs and no scripts or external images.
- [Browser checks](proof/expressions/browser-checks.json): four raw pages, two
  comparison sheets, and eight desktop/phone page views pass. Cards remain in
  first panels. Lettering, navigation, keyboard, contact sheet, Art only, and
  transcripts pass. Requests are file-only; no script exception was reported.
- [Affected website tests](proof/expressions/web-tests.txt): comic discovery and
  lore tests pass, including 28 lore pages and 724 links. No public registration
  or website copy input changed.
- Before the requested commit, [full website CI](proof/expressions/pre-commit-web-ci.txt)
  passed formatting, lint, tests, and the root build. Fresh
  [generation checks](proof/expressions/pre-commit-generation.txt) and
  [20 illustration tests](proof/expressions/pre-commit-unit-tests.txt) also passed.
  No Rust check was needed; no Rust or gameplay source changed.

No release changelog entry is added for this private follow-up. No world fact,
public portrait, shipped behavior, or maintained season event changed.

## Acceptance and next work

The user accepted the restrained amused/wry pair by requesting its commit.
Keep these expressions for the page-1 coffee exchange, not as new defaults
for every appearance. No wider expression pass or publication is authorized.
Story writing remains the main open task; further expression variants are not
its prerequisite.
