# Sharpen the web visual design to an industrial HUD-panel style

- STATUS: OPEN
- PRIORITY: 20
- TAGS: spike,web,design

## Goal

Make the `web/` site feel sharper and more industrial/structured (in the spirit
of factorio.com, not a copy), and less like a generated landing page, while
keeping the current Nova Protocol palette (navy / cyan / amber) and the banner
art. Direction is decided in the spike; this task implements it.

Scope (per the spike's recommended Option B - see the Spike link in Notes):

- Design tokens: add `--radius` (~2px), a machined bevel shadow, and a hard
  (non-blurred) shadow; retire the diffuse glow tokens/halos.
- Corners: shrink radii across the board (cards, hero art, buttons, figures,
  video, nav, blockquote, code, kbd) toward crisp 0-2px.
- Borders as the primary structure: 1px solid, brightening to cyan on
  interaction; add the 2-tone top-highlight/bottom-shade bevel on panels.
- Shadows not glow: replace blurred drop shadows with tight/hard ones or none;
  delete the `0 0 Npx` cyan/amber halos; keep at most a restrained brand glow.
- Buttons: flat fills (no gradient), 1px border, a keycap-style press on
  `:active` instead of a hover lift.
- Hover: remove `translateY` float; use border-brighten / background-tint /
  accent-bar feedback instead.
- Header: drop `backdrop-filter` blur for a solid opaque bar with a strong
  bottom border + thin cyan accent; nav active state as bracket/underline, not
  a rounded pill.
- Hand-crafted framing: replace the emoji card icons (`index.html`) with mono
  index numbers / kicker labels; consistent `//` or `[ ]` mono eyebrows;
  hairline section dividers; a few restrained corner-tick accents on the hero
  and section heads only.
- Verify Rajdhani / JetBrains Mono actually load (webfont), not falling back.

Done when: the site reads sharp and deliberate, the palette and banner are
unchanged, `cd web && npm run ci` is green, and a build render confirms the new
look on the landing page, a devlog post, and the tutorial.

## Notes

- Spike: tasks/20260713-221822/SPIKE.md (frames the "floaty" levers, the
  options considered, and why Option B). Read it before planning.
- Stepless on purpose: `/plan` breaks this into ordered steps when picked up.
- Touch points: `web/src/style.css` (the bulk), `web/src/index.html`,
  `web/src/_header.html`, `web/src/_footer.html`, and the content templates
  (`tutorial.html`, `wiki.html`, `blog.html`, `posts/*.html`) for any markup
  hooks the new styles need. Build/verify with `cd web && npm run ci`.
