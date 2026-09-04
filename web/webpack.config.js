const fs = require("fs");
const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const HtmlPartialsPlugin = require("./webpack-partials");
const CopyPlugin = require("copy-webpack-plugin");
const { docPage, newsPostPage } = require("./markdown");
const {
    discoverComics,
    comicIndexPage,
    comicReaderPage,
} = require("./comic-build");
const { DOC_SECTIONS } = require("./src/docs-manifest");
const getPort = require("get-port");

// PUBLIC_PATH should be "/" for local dev (default) or "/nova-protocol/" for the
// GitHub project-pages deploy, so asset URLs and inter-page links resolve under
// the subpath. The Bevy game is published as a sibling of these pages at
// `<PUBLIC_PATH>play/` (built separately by Trunk); the "Play" links point there.
const publicPath = process.env.PUBLIC_PATH || "/";
const COMICS = discoverComics();

// One HtmlWebpackPlugin per page. `filename` with a trailing `index.html` gives
// clean directory URLs (/news/, /wiki/, ...). `basePath` is read by the
// template (for direct <%= htmlWebpackPlugin.options.basePath %> interpolation)
// and by HtmlPartialsPlugin (for the shared header/footer links).
const page = (chunk, template, filename) =>
    new HtmlWebpackPlugin({
        template,
        filename,
        chunks: [chunk],
        basePath: publicPath,
    });

// Every doc page (both /wiki/ and /create/) comes from src/docs-manifest.js -
// the ONE list that also drives the browser chrome (sidebar, search, tags,
// see-also and index cards in src/docs.ts). Adding a page = its markdown file
// plus one manifest entry; the generated page, crumb, TOC, meta description
// and dev-server rewrite all derive from that entry, so the page list and the
// chrome can no longer drift apart.
//
// The manifest is validated here at config time so a broken entry fails the
// build loudly instead of silently dropping a page from the sidebar.
const validateSection = (section) => {
    const slugs = new Set(section.pages.map((p) => p.slug));
    for (const p of section.pages) {
        const where = `${section.root}/${p.slug}`;
        if (!section.categories.includes(p.category)) {
            throw new Error(
                `docs-manifest: ${where} has unknown category "${p.category}"`
            );
        }
        if (p.parent && !slugs.has(p.parent)) {
            throw new Error(
                `docs-manifest: ${where} has unknown parent "${p.parent}"`
            );
        }
        for (const r of p.related) {
            if (!slugs.has(r)) {
                throw new Error(
                    `docs-manifest: ${where} relates to unknown slug "${r}"`
                );
            }
        }
    }
};
const sectionDocPages = (section) => {
    validateSection(section);
    const bySlug = new Map(section.pages.map((p) => [p.slug, p]));
    const pages = section.pages
        .filter((p) => !p.comingSoon)
        .map((p) => {
            const parent = p.parent ? bySlug.get(p.parent) : undefined;
            return docPage({
                section,
                slug: p.slug,
                mdPath: `${section.mdDir}/${p.md}`,
                title: p.title,
                description: p.summary,
                crumbParent: parent && {
                    slug: parent.slug,
                    title: parent.title,
                },
                toc: p.toc,
                publicPath,
            });
        });
    if (section.landing) {
        pages.push(
            docPage({
                section,
                mdPath: `${section.mdDir}/${section.landing.md}`,
                title: section.landing.title,
                description: section.landing.description,
                landing: true,
                publicPath,
            })
        );
    }
    return pages;
};

// News: markdown under `src/news/<version>.md`, rendered at build time (see
// markdown.js newsPostPage/newsPostShell) into the standalone news article shell
// and served at `/news/<version>/`. News merges the old devlog and release-notes
// sections: ONE post per FEATURE release, newest first. Patch releases are NOT
// given their own post - they fold into their parent feature post's "Point
// releases" section (the terse root CHANGELOG.md keeps every version). Posts
// share the `news` chunk; the news INDEX (news.html) stays hand-authored HTML.
// To add a release: drop `src/news/<version>.md` and add an entry here (newest
// first). `slug` is the version and doubles as the URL segment; date/version
// fill the meta line; description is the head meta; the title comes from the H1.
const NEWS_POSTS = [
    {
        slug: "0.12.0",
        version: "v0.12.0",
        date: "2026-08-31",
        description:
            "Nova Protocol v0.12.0: the editor holds a whole mission - ships, range and script under one scene tree, every node opened by a reflected Inspector, handlers and expressions authored in the panel - and saves it as an ordinary mod. Every key and pad button rebinds from a real Settings panel and the change reaches the flight rig on the spot. Combat learns it happens in a vacuum: torpedoes are dropped inert and lit clear of the hull, a muzzle flash is a ball of burning gas measured in metres, and a detonation lights the hulls around it.",
    },
    {
        slug: "0.11.0",
        version: "v0.11.0",
        date: "2026-08-22",
        description:
            "Nova Protocol v0.11.0: ships wear damage and tear into drifting wrecks, Kinetic and Pierce rounds travel differently, point defence answers weaving torpedoes per mount, derived skins give hulls four authored identities, the editor becomes visual, and gun rounds leave the rigid-body simulation.",
    },
    {
        slug: "0.10.0",
        version: "v0.10.0",
        date: "2026-08-13",
        description:
            "Nova Protocol v0.10.0: Nova gains predicate-driven automation and proof-bearing reports, semantic Racer and cargo-ship parts with explicit structural mates, typed scenario queries and exact lifecycle events, authored lighting and mass-based gravity, reproducible screenshots, and a clearer workspace architecture.",
    },
    {
        slug: "0.9.0",
        version: "v0.9.0",
        date: "2026-08-01",
        description:
            "Nova Protocol v0.9.0: the cockpit becomes a real NOVA OS ship computer, with a CRT terminal drawer, command output, shell input, app takeover surfaces, scanlines, bloom, sound, power controls and a full phosphor HUD language. Combat reads faster with allegiance markers, neutralized ships, contextual chips and clearer lock decay; scenarios browse by campaign headers; the website adopts the same phosphor skin; assets preload with credited fonts, key glyphs and UI sounds; and nova_probe runs are now profile-sandboxed.",
    },
    {
        slug: "0.8.0",
        version: "v0.8.0",
        date: "2026-07-23",
        description:
            "Nova Protocol v0.8.0: the base campaign is finished and finds its voice - two new chapters (Lifeline's convoy defense and the Final Tally gravity-well finale) close the arc, and the whole mainline briefs you over the comms and breathes between beats. The Ledger story mod (now 1.12.0) grows a real stealth run, a forking finale, and a fifth reward chapter you fly a torpedo-armed gunship into; Gauntlet becomes a time-trial on the new HudReadout action; and the dev tooling grows the nova_probe run-harness that verifies an autopilot playthrough is still correct, a unified content lint with per-mod reports, and an ephemeral docs model.",
    },
    {
        slug: "0.7.0",
        version: "v0.7.0",
        date: "2026-07-18",
        description:
            "Nova Protocol v0.7.0: scenarios can declare victory or defeat with a real outcome frame, a second base-campaign chapter (Broadside) and a four-chapter campaign mod (The Ledger) on the portal, smarter fights (cover, auto-reload ammo, earned locks, multi-barrel turrets), RCS docking thrusters, a real Settings menu, and arbitrary joint-tree turrets plus self:// / dep:// asset schemes for modders.",
    },
    {
        slug: "0.6.0",
        version: "v0.6.0",
        date: "2026-07-16",
        description:
            "Nova Protocol v0.6.0: a static mod portal and an in-game Explore online tab install, update and uninstall mods over the wire on native and web, mod dependencies resolve end to end, a main-menu Scenarios picker, and particles return to the web build on WebGPU.",
    },
    {
        slug: "0.5.0",
        version: "v0.5.0",
        date: "2026-07-13",
        description:
            "Nova Protocol v0.5.0: deliberate CTRL-to-sweep radar locking with a live target viewfinder and kill cam, the Shakedown Run tutorial, typed damage against per-section resistances, a main menu and pause screen, and a landing site on the web (with the v0.5.1 and v0.5.2 point releases).",
    },
    {
        slug: "0.4.0",
        version: "v0.4.0",
        date: "2026-07-10",
        description:
            "Nova Protocol v0.4.0: proportionally-navigated guided torpedoes, a full targeting arc with per-section fine-lock, turret auto-aim with true intercept lead, a faction model, an AI combat wave with a behavior state machine, a center-of-mass flight-assist overhaul, and the first audio and combat juice (with the v0.4.1 point release).",
    },
    {
        slug: "0.3.0",
        version: "v0.3.0",
        date: "2025-11-29",
        description:
            "Nova Protocol v0.3.0: OnEnter/OnExit zone events for richer scenarios, the torpedo bay section with area-of-effect blast damage, a per-section health system, and sharper directional and thruster shaders (with the v0.3.1 Bevy 0.19 point release).",
    },
    {
        slug: "0.2.0",
        version: "v0.2.0",
        date: "2025-11-08",
        description:
            "Nova Protocol v0.2.0: a data-driven game-events and queue system, the first scenario and modding capabilities, and procedurally generated asteroids with dynamic destruction (with a video devlog and the v0.2.1 point release).",
    },
    {
        slug: "0.1.0",
        version: "v0.1.0",
        date: "2025-10-21",
        description:
            "Nova Protocol v0.1.0, the first release: thruster-driven modular ships, PD-controlled mouse steering, turrets that shoot, and a health system that shatters sections into chunks. Includes the very first video devlog.",
    },
];
const newsPage = (p) =>
    newsPostPage({ ...p, mdPath: `src/news/${p.slug}.md`, publicPath });

// Redirect stubs for the retired /blog/ and /changelog/ URLs -> the merged
// /news/ posts (patch versions fold into their parent feature post). Each emits
// a tiny meta-refresh + canonical page (no chunks, no header/footer) so old
// links and bookmarks keep resolving after the merge.
const redirectHtml = (to) =>
    `<!doctype html>
<html lang="en">
    <head>
        <meta charset="UTF-8" />
        <meta http-equiv="refresh" content="0; url=${to}" />
        <link rel="canonical" href="${to}" />
        <title>Moved</title>
    </head>
    <body>
        <p>This page moved to <a href="${to}">${to}</a>.</p>
    </body>
</html>`;
const REDIRECTS = [
    ["blog", "news"],
    ["changelog", "news"],
    ["blog/devlog-1-modular-ships-and-first-combat", "news/0.1.0"],
    ["blog/devlog-2-objectives-enemy-ai-and-asteroids", "news/0.2.0"],
    ["blog/devlog-3-zones-torpedoes-and-blast-damage", "news/0.3.0"],
    ["blog/devlog-4-guided-torpedoes-targeting-and-enemy-ai", "news/0.4.0"],
    ["blog/devlog-5-radar-locking-shakedown-and-the-web", "news/0.5.0"],
    ["changelog/0.1.0", "news/0.1.0"],
    ["changelog/0.2.0", "news/0.2.0"],
    ["changelog/0.2.1", "news/0.2.0"],
    ["changelog/0.3.0", "news/0.3.0"],
    ["changelog/0.3.1", "news/0.3.0"],
    ["changelog/0.4.0", "news/0.4.0"],
    ["changelog/0.4.1", "news/0.4.0"],
    ["changelog/0.5.0", "news/0.5.0"],
    ["changelog/0.5.1", "news/0.5.0"],
    ["changelog/0.5.2", "news/0.5.0"],
    ["changelog/0.6.0", "news/0.6.0"],
];
const redirectPage = ([from, to]) =>
    new HtmlWebpackPlugin({
        filename: `${from}/index.html`,
        chunks: [],
        inject: false,
        templateContent: redirectHtml(publicPath + to + "/"),
    });

// Dev-server port. `scripts/serve-web.sh` allocates all three preview ports up
// front (site, game, portal) and passes them down as NOVA_UI_PORT/GAME_DEV_URL/
// MODS_DEV_URL, because the proxies below have to know where the other two
// servers landed. A bare `npm run serve` gets no such env, so it picks its own
// free port in 7000-7999 - several worktrees can then serve at once without
// fighting over a fixed port. There is no hardcoded fallback on purpose: a busy
// fallback port fails later and more confusingly than failing here.
const resolveUiPort = async () => {
    const preset = process.env.NOVA_UI_PORT;
    if (preset) {
        const port = Number(preset);
        if (!Number.isInteger(port) || port < 1 || port > 65535) {
            throw new Error(`NOVA_UI_PORT is not a valid port: ${preset}`);
        }
        return port;
    }
    return getPort.default({ port: getPort.portNumbers(7000, 7999) });
};

module.exports = async (env, argv) => {
    // Only `webpack serve` needs a port; skipping the scan keeps `npm run build`
    // (and CI) from probing sockets it will never listen on.
    const uiPort = env && env.WEBPACK_SERVE ? await resolveUiPort() : undefined;

    return {
        entry: {
            index: "./src/index.ts",
            docs: "./src/docs.ts",
            news: "./src/news.ts",
            story: "./src/story.ts",
        },
        output: {
            path: path.resolve(__dirname, "dist"),
            filename: "[name].js",
            assetModuleFilename: "assets/[name][ext]",
            clean: true,
            publicPath: publicPath,
        },
        plugins: [
            page("index", "src/index.html", "index.html"),
            page("docs", "src/wiki.html", "wiki/index.html"),
            ...DOC_SECTIONS.flatMap(sectionDocPages),
            page("news", "src/news.html", "news/index.html"),
            comicIndexPage(COMICS, publicPath),
            ...COMICS.map((comic) => comicReaderPage(comic, publicPath)),
            ...NEWS_POSTS.map(newsPage),
            ...REDIRECTS.map(redirectPage),
            new CopyPlugin({
                patterns: [
                    { from: "src/assets", to: "assets" },
                    { from: "src/favicon.svg", to: "favicon.svg" },
                    // Easter egg: the self-contained UI-rework PoCs live in
                    // `web/design/` (their source of truth). Copy them verbatim into
                    // the build at secret, unlinked routes rather than committing a
                    // second copy under `src/`. The 5x brand-click (src/site.ts) opens
                    // `/nova-menu/`; New Game -> `/nova-hud/`; the HUD's NOVA OS button
                    // -> `/nova-os/`. The menu + CRT PoCs have no relative asset refs
                    // and render as-is under any publicPath; the HUD PoC references the
                    // input-prompt key glyphs, which live in the game asset tree
                    // (`assets/input-prompts/`), so that folder is copied alongside.
                    {
                        from: "design/nova_ui_rework_poc.html",
                        to: "nova-menu/index.html",
                    },
                    {
                        from: "design/hud_rework_poc.html",
                        to: "nova-hud/index.html",
                    },
                    {
                        from: "../assets/input-prompts",
                        to: "nova-hud/assets/input-prompts",
                    },
                    {
                        from: "design/nova_os_terminal_poc.html",
                        to: "nova-os/index.html",
                    },
                ],
            }),
            new HtmlPartialsPlugin({ basePath: publicPath }),
        ],
        resolve: {
            extensions: [".ts", ".tsx", ".js"],
        },
        module: {
            rules: [
                {
                    test: /\.tsx?$/,
                    use: "ts-loader",
                    exclude: /node_modules/,
                },
                {
                    test: /\.css$/i,
                    use: ["style-loader", "css-loader", "postcss-loader"],
                },
            ],
        },
        mode: "development",
        devServer: {
            // The developer book is a sibling build (mdbook, repo-root book/)
            // served under /dev/ like the deploy. Static entries resolve before
            // historyApiFallback, so book files never fall through to the SPA
            // fallback. Guarded: a bare `npm run serve` without a book build
            // still starts; `scripts/serve-web.sh` builds and watches it.
            static: [
                path.join(__dirname, "dist"),
                ...(fs.existsSync(path.join(__dirname, "..", "book"))
                    ? [
                          {
                              directory: path.join(__dirname, "..", "book"),
                              publicPath: "/dev",
                          },
                      ]
                    : []),
            ],
            port: uiPort,
            // Two proxies reproduce the published sibling layout (site at /,
            // game at /play/, portal at /mods/) on this one origin. Both are
            // registered before historyApiFallback, so neither path can fall
            // through to the SPA fallback and answer with landing-page HTML.
            //
            // /play -> a running `trunk serve` (the Bevy WASM game). Without it
            // the Play button lands on the landing page again. The game uses
            // relative asset URLs, so stripping the /play prefix is all it
            // needs; `ws: true` carries Trunk's autoreload socket.
            //
            // /mods -> a running `scripts/serve-mods.sh` (the generated static
            // portal). The wasm build derives its portal base from
            // window.location - at /play/ it steps out and fetches <origin>/mods
            // - so the Explore tab hits THIS server, not trunk's. Serving it
            // here keeps it same-origin, with no `?portal=` override and no CORS.
            //
            // `scripts/serve-web.sh` sets both targets. The defaults only cover
            // a hand-started stack: trunk's own [serve] port, and a portal
            // pinned with `NOVA_MODS_PORT=9000 scripts/serve-mods.sh`.
            proxy: [
                {
                    context: ["/play"],
                    target: process.env.GAME_DEV_URL || "http://localhost:8080",
                    pathRewrite: { "^/play": "" },
                    changeOrigin: true,
                    ws: true,
                },
                {
                    context: ["/mods"],
                    target: process.env.MODS_DEV_URL || "http://localhost:9000",
                    changeOrigin: true,
                },
            ],
            historyApiFallback: {
                rewrites: [
                    // Easter-egg routes: resolve /nova-menu, /nova-hud and /nova-os
                    // (with or without a trailing slash) to the copied PoCs during
                    // `webpack serve`. Order before the broader rewrites below.
                    { from: /^\/nova-menu/, to: "/nova-menu/index.html" },
                    { from: /^\/nova-hud/, to: "/nova-hud/index.html" },
                    { from: /^\/nova-os/, to: "/nova-os/index.html" },
                    // Doc pages, deepest slugs first so the more specific path
                    // wins (/wiki/sections/hull before /wiki/sections), then the
                    // section root as the fallback.
                    ...DOC_SECTIONS.flatMap((section) => [
                        ...section.pages
                            .filter((p) => !p.comingSoon)
                            .map(({ slug }) => ({
                                from: new RegExp(`^/${section.root}/` + slug),
                                to: `/${section.root}/` + slug + "/index.html",
                                depth: slug.split("/").length,
                            }))
                            .sort((a, b) => b.depth - a.depth)
                            .map(({ from, to }) => ({ from, to })),
                        {
                            from: new RegExp(`^/${section.root}`),
                            to: `/${section.root}/index.html`,
                        },
                    ]),
                    ...NEWS_POSTS.map(({ slug }) => ({
                        from: new RegExp("^/news/" + slug),
                        to: "/news/" + slug + "/index.html",
                    })),
                    { from: /^\/news/, to: "/news/index.html" },
                    ...COMICS.map((comic) => ({
                        from: new RegExp(`^/story/${comic.path}`),
                        to: `/story/${comic.path}/index.html`,
                    })),
                    { from: /^\/story/, to: "/story/index.html" },
                    // Retired sections: the physical redirect stubs under
                    // dist/blog|changelog are served directly; these fallbacks catch
                    // any sub-path that misses a stub and bounce it to the index.
                    ...REDIRECTS.map(([from]) => ({
                        from: new RegExp("^/" + from.replace(/[.]/g, "\\$&")),
                        to: "/" + from + "/index.html",
                    })),
                    { from: /^\/blog/, to: "/blog/index.html" },
                    { from: /^\/changelog/, to: "/changelog/index.html" },
                ],
            },
        },
        watchOptions: {
            ignored: ["**/node_modules/**", "**/dist/**"],
        },
    };
};
