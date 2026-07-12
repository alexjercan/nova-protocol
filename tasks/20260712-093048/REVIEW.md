## Round 1

VERDICT: APPROVE

- [x] R1.1 (MINOR) web/src/site.ts - Active-nav detection breaks under the `/nova-protocol/` subpath. `isHome` is computed as `href.replace(/\/+$/,"") === ""`, but under the deploy subpath the Home link's pathname is `/nova-protocol` (not `""`), so `isHome` is always false. The Home link is then matched by the generic `path.startsWith("/nova-protocol")` branch, which is true on every page - so Home is marked `aria-current="page"` and highlighted on all pages, and on inner pages both Home and the current section light up. Fix: derive the site base (e.g. strip the known basePath, or compare against the Home link's full normalized pathname) so the home test is `path === homeHref` and inner links use `path.startsWith(hrefWithoutTrailingSlash + "/")`. Works fine at root (`/`) locally, which is why it passed the local screenshot; only wrong under the subpath deploy.
- [x] R1.2 (MINOR) .github/workflows/deploy-page.yaml - No `.nojekyll` is emitted to the published site. GitHub Pages runs Jekyll by default, which drops files/dirs whose names begin with `_`. The current build output has no leading-underscore paths (the `_header.html`/`_footer.html` partials are source-only and inlined at build time; Trunk/wasm-bindgen emit `index_bg.wasm` etc. where the underscore is not leading), so nothing breaks today. But this is fragile: add a `touch site/.nojekyll` in the Assemble step to make the static site immune to Jekyll and slightly faster to publish. Low risk, cheap insurance.
- [~] R1.3 (NIT) web/src/{index,blog,post,tutorial,wiki}.ts - Five identical 3-line entry files (`import "./style.css"; import { initSite } from "./site"; initSite();`). This mirrors the reference football-guessr multi-page shape and is harmless, but a single shared entry imported per page would remove the duplication. Leave as-is if matching the reference stack is intended.

### Round 1 responses

- R1.1 fixed: `site.ts` now derives the site root from the brand link's pathname
  and marks a link active only on an exact match, or when it is a non-root prefix
  of the current path (so a blog post keeps Blog active). Verified with a
  simulation across home/tutorial/wiki/blog-index/blog-post under both the
  `/nova-protocol/` subpath and root `/`: exactly the right single link is active
  on each page (Home no longer lights up everywhere).
- R1.2 fixed: the Assemble step now `touch site/.nojekyll`.
- R1.3 (NIT) left as-is: the five one-line entries intentionally mirror the
  football-guessr multi-page reference stack; folding them saves three lines at
  the cost of diverging from the reference shape. Not worth it.

### Notes (verified, no action needed)
- Deploy layout is correct: `PUBLIC_PATH=/nova-protocol/` is a job-level env; webpack.config.js reads `process.env.PUBLIC_PATH` for both `output.publicPath` and every page's `basePath`. `cp -r web/dist/. site/` puts the landing site at gh-pages root (served at `/nova-protocol/`), and `cp -r dist/. site/play/` puts the game at `/nova-protocol/play/`. wasm-opt runs on `dist/*.wasm` before the copy. Correct.
- `web/package-lock.json` is committed, `lockfileVersion: 3`, root name matches package.json, and every devDependency has both a root-devDeps entry and an installed `node_modules/*` entry - `npm ci` will succeed and be in sync.
- Game position-independence holds: `Trunk.toml` uses `public_url = "./"`, so wasm-bindgen output paths are relative and resolve correctly from `/nova-protocol/play/`. The non-Trunk `<link rel="icon" href="icon.ico">` is also relative and resolves under the play subdir.
- Base paths are consistent everywhere: templates use `<%= htmlWebpackPlugin.options.basePath %>` (banner img, favicon, CTAs, inter-page links) and partials use `<%= basePath %>` (nav/footer/brand), both fed by `publicPath`. No hardcoded `/` roots found. The Play CTA points at `<basePath>play/`.
- devServer `historyApiFallback` rewrites match the emitted filenames (`blog/index.html`, `blog/building-nova-protocol/index.html`, `tutorial/`, `wiki/`); the more-specific post rewrite is ordered before the generic `/blog` rewrite. Correct.
- Content accuracy spot-checked against crates/nova_gameplay/src/input/player.rs + camera_controller.rs + hud/mod.rs: W/Space/RT thrust, G/North GOTO, O/DPadDown ORBIT, X/East STOP, Z/West CANCEL, brackets + DPadLeft/Right component cycle, Ctrl+scroll / DPadUp ship-lock cycle, grave/Backquote HUD cycle, Alt/LeftTrigger free-look, RMB/LeftTrigger2 combat mode - all match the actual bindings.
- eslint/tsconfig/package.json scripts are coherent (typechecked eslint with projectService, config files ignored; `types:["node"]` backed by @types/node). CHANGELOG and README diffs are accurate and point at the live `/nova-protocol/` URLs.
