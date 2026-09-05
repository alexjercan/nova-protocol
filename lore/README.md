# Lore

The story bible of Nova Protocol: the world before the campaign, the people,
the places and ships, the company's paper, and the Season 1 timeline. The comic
and the scenarios are written from this book. It is not published: every page
is a spoiler.

Build and read it from the repository root:

    nix develop --command mdbook serve lore --open

`src/` holds the pages, `theme/crt.css` the CRT look, and `sketches.py` draws
`src/sketches/*.svg` on the comic's vocabulary in `art/comics/crt.py`:

    python3 lore/sketches.py

Every page ends with `Hooks`, what it gives the story, and `Open questions`,
what is not decided. Dates count from Season 1: Y0 is the campaign year, Y-4 is
four years before it.
