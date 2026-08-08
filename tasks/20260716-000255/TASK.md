# Fix km->u unit slip in targeting-radar.md lock ranges

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: bug, docs, web

## The fix

`web/src/wiki/targeting-radar.md:43` used "20 km" / "2.5 km" for lock ranges
while the whole wiki + glossary teach distance in `u`. Changed to "20000 u" /
"2500 u" (matches input/targeting.rs: ship lock 20000, torpedo lock 2500).

Done. Source: spike 20260715-235232.
