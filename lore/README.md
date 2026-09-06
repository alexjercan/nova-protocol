# Lore

A CRT setting book for Nova Protocol. The entry point is `src/index.md`.

[Setting](src/setting.md) records the approved world context and marks the
remaining open decisions. No story direction or plot has been chosen. Do not
recover earlier lore from Git.

The [Encyclopedia](src/encyclopedia/index.md) develops short, linked reference
articles from that setting. Add individual companies, places, and institutions
as their identities are approved. Do not fill missing names or specifications
for completeness.

## Technical reference

[Technical specification](TECHNICAL_SPEC.md) records mechanics, configured
values, derived estimates, and implementation gaps at its stated implementation
revision. It is a working reference, not a book chapter or story canon. Recheck
the relevant code before relying on a mechanic for a campaign.

## Image generation

Regenerate only the Preface's image from the repository root:

```sh
python3 lore/tools/generate_art.py
```

The generator uses only the Python standard library. Commit both the generator
and its SVG output. Change the generator rather than hand-editing its output.

Encyclopedia images are directly authored SVG files in
`src/images/encyclopedia/`. They do not use or modify the Preface generator.
Caption relationship diagrams as references and atmospheric illustrations as
visual concepts. A concept image does not establish a location, design, or
scale without separate approval.

Use direct SVG for diagrams, emblems, and simpler illustrations. Use a script
when repeated geometry or regeneration provides a clear benefit; do not build
a generator for every image.

Do not add or run book tests.
