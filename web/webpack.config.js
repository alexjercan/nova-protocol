const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const HtmlPartialsPlugin = require("./webpack-partials");
const CopyPlugin = require("copy-webpack-plugin");
const { wikiDocPage, newsPostPage } = require("./markdown");

// PUBLIC_PATH should be "/" for local dev (default) or "/nova-protocol/" for the
// GitHub project-pages deploy, so asset URLs and inter-page links resolve under
// the subpath. The Bevy game is published as a sibling of these pages at
// `<PUBLIC_PATH>play/` (built separately by Trunk); the "Play" links point there.
const publicPath = process.env.PUBLIC_PATH || "/";

// One HtmlWebpackPlugin per page. `filename` with a trailing `index.html` gives
// clean directory URLs (/blog/, /tutorial/, ...). `basePath` is read by the
// template (for direct <%= htmlWebpackPlugin.options.basePath %> interpolation)
// and by HtmlPartialsPlugin (for the shared header/footer links).
const page = (chunk, template, filename) =>
    new HtmlWebpackPlugin({
        template,
        filename,
        chunks: [chunk],
        basePath: publicPath,
    });

// Every wiki page is markdown under `src/wiki/`, rendered at build time (see
// markdown.js) and served at `/wiki/<slug>/`; all share the `wiki` chunk (the
// manifest-driven sidebar/search/see-also from wiki.ts + wiki-pages.ts). To add
// a page: drop the `.md` under `src/wiki/`, add an entry here, and add a manifest
// entry in src/wiki-pages.ts. Keep this list in sync with wiki-pages.ts.
// Children are listed before their parent so the dev-server rewrites match the
// more specific path first (/wiki/sections/hull before /wiki/sections).
const SECTIONS_CRUMB = { slug: "sections", title: "Ship sections" };
const WIKI_DOC_PAGES = [
    // Player pages (children before the sections parent for rewrite ordering).
    {
        slug: "sections/hull",
        md: "sections/hull.md",
        title: "Hull",
        crumbParent: SECTIONS_CRUMB,
    },
    {
        slug: "sections/controller",
        md: "sections/controller.md",
        title: "Controller",
        crumbParent: SECTIONS_CRUMB,
    },
    {
        slug: "sections/thruster",
        md: "sections/thruster.md",
        title: "Thruster",
        crumbParent: SECTIONS_CRUMB,
    },
    {
        slug: "sections/turret",
        md: "sections/turret.md",
        title: "Turret",
        crumbParent: SECTIONS_CRUMB,
    },
    {
        slug: "sections/torpedo-bay",
        md: "sections/torpedo-bay.md",
        title: "Torpedo bay",
        crumbParent: SECTIONS_CRUMB,
    },
    {
        slug: "getting-started",
        md: "getting-started.md",
        title: "Your first flight",
    },
    { slug: "glossary", md: "glossary.md", title: "Glossary" },
    { slug: "sections", md: "sections.md", title: "Ship sections" },
    { slug: "keybinds", md: "keybinds.md", title: "Keybinds" },
    { slug: "hud", md: "hud.md", title: "HUD" },
    { slug: "settings", md: "settings.md", title: "Settings" },
    {
        slug: "flight-autopilot",
        md: "flight-autopilot.md",
        title: "Flight & autopilot",
    },
    {
        slug: "targeting-radar",
        md: "targeting-radar.md",
        title: "Targeting & radar",
    },
    {
        slug: "combat-weapons",
        md: "combat-weapons.md",
        title: "Combat & weapons",
    },
    { slug: "gravity-wells", md: "gravity-wells.md", title: "Gravity wells" },
    { slug: "factions", md: "factions.md", title: "Factions" },
    { slug: "scenarios", md: "scenarios.md", title: "Scenarios" },
    { slug: "modding", md: "modding.md", title: "Modding" },
    // Developer pages (markdown under src/wiki/dev/).
    {
        slug: "dev/development",
        md: "dev/development.md",
        title: "Building & running",
    },
    {
        slug: "dev/keeping-docs-in-sync",
        md: "dev/keeping-docs-in-sync.md",
        title: "Keeping docs in sync",
    },
    {
        slug: "dev/architecture",
        md: "dev/architecture.md",
        title: "Architecture",
    },
    {
        slug: "dev/sections",
        md: "dev/sections.md",
        title: "Ship sections (internals)",
    },
    {
        slug: "dev/scenario-system",
        md: "dev/scenario-system.md",
        title: "Scenario engine",
    },
    {
        slug: "dev/modding-ron",
        md: "dev/modding-ron.md",
        title: "Modding data format (RON)",
    },
    { slug: "dev/mod-portal", md: "dev/mod-portal.md", title: "Mod portal" },
    {
        slug: "dev/project-tour",
        md: "dev/project-tour.md",
        title: "Project tour",
    },
    {
        slug: "dev/guide-add-section",
        md: "dev/guide-add-section.md",
        title: "Add a ship section",
    },
    {
        slug: "dev/guide-extend-scenarios",
        md: "dev/guide-extend-scenarios.md",
        title: "Extend the scenario engine",
    },
    {
        slug: "dev/guide-author-scenario",
        md: "dev/guide-author-scenario.md",
        title: "Author a scenario (RON)",
    },
    {
        slug: "dev/guide-author-section",
        md: "dev/guide-author-section.md",
        title: "Author a section (RON)",
    },
    {
        slug: "dev/guide-make-a-mod",
        md: "dev/guide-make-a-mod.md",
        title: "Make and publish a mod",
    },
];
const docPage = ({ slug, md, title, crumbParent }) =>
    wikiDocPage({
        slug,
        mdPath: `src/wiki/${md}`,
        title,
        crumbParent,
        publicPath,
    });

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

const config = {
    entry: {
        index: "./src/index.ts",
        tutorial: "./src/tutorial.ts",
        wiki: "./src/wiki.ts",
        news: "./src/news.ts",
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
        page("tutorial", "src/tutorial.html", "tutorial/index.html"),
        page("wiki", "src/wiki.html", "wiki/index.html"),
        ...WIKI_DOC_PAGES.map(docPage),
        page("news", "src/news.html", "news/index.html"),
        ...NEWS_POSTS.map(newsPage),
        ...REDIRECTS.map(redirectPage),
        new CopyPlugin({
            patterns: [
                { from: "src/assets", to: "assets" },
                { from: "src/favicon.svg", to: "favicon.svg" },
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
        static: path.join(__dirname, "dist"),
        port: 8090,
        // Forward /play/ to a running `trunk serve` (the Bevy WASM game) so the
        // Play button works during live site development. Start the game first
        // with `trunk serve` at the repo root (defaults to :8080), then this
        // dev server; without it, /play/ has nothing to serve and would fall
        // through to the history fallback (the landing page). Registered before
        // historyApiFallback, so /play never reaches the SPA fallback. The game
        // uses relative asset URLs, so stripping the /play prefix is all it
        // needs. Override the target with GAME_DEV_URL if trunk runs elsewhere.
        proxy: [
            {
                context: ["/play"],
                target: process.env.GAME_DEV_URL || "http://localhost:8080",
                pathRewrite: { "^/play": "" },
                changeOrigin: true,
                ws: true,
            },
        ],
        historyApiFallback: {
            rewrites: [
                { from: /^\/tutorial/, to: "/tutorial/index.html" },
                ...WIKI_DOC_PAGES.map(({ slug }) => ({
                    from: new RegExp("^/wiki/" + slug),
                    to: "/wiki/" + slug + "/index.html",
                })),
                { from: /^\/wiki/, to: "/wiki/index.html" },
                ...NEWS_POSTS.map(({ slug }) => ({
                    from: new RegExp("^/news/" + slug),
                    to: "/news/" + slug + "/index.html",
                })),
                { from: /^\/news/, to: "/news/index.html" },
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

module.exports = config;
