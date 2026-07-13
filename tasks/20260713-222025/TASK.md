# Sharpen the web visual design to an industrial HUD-panel style

- STATUS: CLOSED
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

## Steps

- [x] Foundation: add `--radius` (~2px), `--bevel` (inset top-highlight +
      bottom-shade) and `--shadow-hard` tokens; retire `--shadow-glow-cyan` and
      soften `--shadow-panel`. Confirm Rajdhani / JetBrains Mono actually load
      (add the webfont `<link>`/`@font-face` if they are silently falling back).
- [x] Header: drop `backdrop-filter` blur for a solid opaque `--space-0` bar
      with a strong bottom border + thin cyan accent; square the nav items and
      give the active/hover state a bracket or underline instead of a pill.
- [x] Buttons: flat solid cyan primary / transparent amber ghost, 1px border,
      near-square corners, a keycap-style `:active` press; remove the hover
      `translateY` lift and the gradient/glow.
- [x] Cards + grids: crisp 1px border + machined bevel, `--radius` corners,
      hover = border-brighten + accent (no float). Add a mono kicker/number
      treatment hook for the feature cards.
- [x] Hero + sections: shrink the hero-art radius and drop its cyan halo; tame
      the amber radial wash; add structural framing (mono eyebrows, hairline
      section dividers, a couple of restrained corner-tick accents on hero /
      section heads only).
- [x] Content surfaces: apply the sharper radii/borders (glow -> hard
      definition) to `.figure`, `.video-embed`, `.prose blockquote/code/kbd`,
      `.post-footer`, `.controls`, `.post-list`, `.site-footer`.
- [x] Markup: replace the emoji card icons in `index.html` with mono index
      numbers / kicker labels; wire any header/footer/section accent hooks the
      new CSS needs. Keep the banner art.
- [x] Build + verify: `cd web && npm run ci` green; render and eyeball the
      landing page, a devlog post, and the tutorial for the sharper look.

## Notes

- Spike: tasks/20260713-221822/SPIKE.md (frames the "floaty" levers, the
  options considered, and why Option B). Read it before planning.
- Stepless on purpose: `/plan` breaks this into ordered steps when picked up.
- Touch points: `web/src/style.css` (the bulk), `web/src/index.html`,
  `web/src/_header.html`, `web/src/_footer.html`, and the content templates
  (`tutorial.html`, `wiki.html`, `blog.html`, `posts/*.html`) for any markup
  hooks the new styles need. Build/verify with `cd web && npm run ci`.
