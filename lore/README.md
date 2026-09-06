# Lore

A CRT, wiki-style setting book for Nova Protocol. Start with
[Introduction](src/index.md), then [Language and dialogue](src/language.md).
The [Atlas](src/atlas.md) holds the world reference. The first story brief,
[The Fall of Kestrel](src/story/fall-of-kestrel.md), follows the connected
historical episode through the start of Y0, not everything before that date.

## Scope and authority

The accepted foundation recorded in this book and the owner's subsequent
decisions are the story authority. Do not recover retired story pages from
Git or consult old tasks, scenarios, implementation code, comics, or art for
story facts. They are not material to reconcile with the current setting.
Repository working instructions still apply; implementation details are not
canon.

Keep new work in `lore/`. Do not modify game or web content, create missions,
or prescribe a future campaign, revelation, battle, or ending. Use no task
machinery unless tracked work is requested.

Keep authorial truth, official accounts, individual knowledge, suspicion, and
proof separate. The [story brief](src/story/fall-of-kestrel.md#truth-account-and-knowledge)
holds the incident's complete boundary. Only Calloway and the Board know the
attack's real purpose at the time; Pell and the placement crew do not.

New dates, technologies, contracts, laws, customs, personal histories, voices,
and visual designs remain explicit proposals until reviewed. The
[review guide](src/questions/index.md) identifies consequential decisions and
links the detailed questions. Update dependent articles when an owner decision
closes or changes a question.

## Ownership of information

- `src/language.md`: terminology, titles, naming, register, and dialogue guidance.
- `src/atlas.md`: summary hub, not a duplicate of every detailed article.
- `src/locations/`, `src/ships/`, `src/characters/`, `src/factions/`: detailed
  entries, including local relationship accounts and open questions.
- `src/atlas/timeline.md`: established order and explicitly proposed years.
- `src/atlas/relationships.md`: comparison at the end of the Atlas.
- `src/story/fall-of-kestrel.md`: motives, established causal history, consequences,
  proposed themes and motifs, and gaps. No prescribed scenes or future arcs.
- `src/questions/`: review priorities and the wider unresolved catalogue.
- `src/SUMMARY.md`: reading order. Unlinked entries group Locations, Ships,
  Characters, and Factions; they are not missing draft articles.

Character articles own the detailed personality and voice proposals. The Atlas
and language tables summarise them. A portrait does not supply a rank, age,
ancestry, or additional role in the Shelter incident. Halloran's assignment to
Meridian remains undecided.

## Presentation and images

`book.toml` configures mdBook; `theme/crt.css` provides the shared palette,
responsive infoboxes, navigation, captions, and proposal notes. The palette is
not a faction colour convention.

The original selected CRT portrait was a visual reference only. The generator
does not read it or import character facts. `tools/generate_art.py` uses only
the Python standard library and writes the checked-in SVGs under `src/images/`:

- Three 1600 x 1000 industrial concept scenes.
- Four 1200 x 1400 character portrait proposals.
- One 1600 x 1600 timeline diagram with labelled fixed, undated, and proposed
  material. Spacing is not a measure of elapsed time.

When chronology is reviewed, update the timeline's text and drawing labels
together. Presentation dimensions are pixels, not dimensions in the setting.
Generated mdBook output goes to `lore/book/` and is ignored by Git.

## Build and read

Run from the repository root:

```sh
nix develop --command mdbook build lore
nix develop --command mdbook serve lore --open
```

Open `lore/book/index.html` for the built Introduction. The book provides
sidebar navigation, search, local contents, and cross-article links. Its source
must remain complete; mdBook is configured not to create missing page stubs.

## Regenerate and review

```sh
python3 lore/tools/generate_art.py
nix develop --command mdbook build lore
```

Generation is explicit, not a preprocessor; the book can build with the
checked-in SVGs alone. Review the affected rendered pages and images after a
change. Use small, relevant read-only inspections between batches. Do not add
or run book tests or restore the deleted test suite. Keep generated review
artifacts under `lore/book/`, not in the maintained source directories.
