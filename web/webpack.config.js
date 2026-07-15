const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const HtmlPartialsPlugin = require("./webpack-partials");
const CopyPlugin = require("copy-webpack-plugin");
const { wikiDocPage, blogPostPage } = require("./markdown");

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

// Blog devlog posts: markdown under `src/posts/`, rendered at build time (see
// markdown.js blogPostPage/blogPostShell) into the standalone blog article shell
// and served at `/blog/<slug>/`. They share the `post` chunk. The blog INDEX
// (blog.html) stays hand-authored HTML. To add a post: drop `src/posts/<slug>.md`
// and add an entry here (newest first). date/version fill the meta line;
// description is the head meta.
const BLOG_POSTS = [
    {
        slug: "devlog-5-radar-locking-shakedown-and-the-web",
        title: "Devlog #5: radar locking, a tutorial, and a home on the web",
        date: "2026-07-13",
        version: "v0.5.0",
        description:
            "Nova Protocol v0.5.0: deliberate CTRL-to-sweep radar locking with stance-driven slots, a live magnified target viewfinder with a kill cam, the Shakedown Run tutorial, typed damage against per-section resistances, a main menu and pause screen, HUD visibility levels, richer objective conveyance, and a brand-new landing site on the web.",
    },
    {
        slug: "devlog-4-guided-torpedoes-targeting-and-enemy-ai",
        title: "Devlog #4: guided torpedoes, targeting and an enemy that fights back",
        date: "2026-07-10",
        version: "v0.4.0",
        description:
            "Nova Protocol v0.4.0: proportionally-navigated guided torpedoes, a full targeting arc with per-section fine-lock, turret auto-aim with true intercept lead, a faction/relation model, an AI combat wave with a real behavior state machine, a flight-assist overhaul that balances thrust through the live center of mass, and the game's first audio and combat juice.",
    },
    {
        slug: "devlog-3-zones-torpedoes-and-blast-damage",
        title: "Devlog #3: zones, torpedoes and blast damage",
        date: "2025-11-29",
        version: "v0.3.0",
        description:
            "Nova Protocol v0.3.0: OnEnter/OnExit lifecycle events and a zone-entry trigger for richer scenarios, the first area-of-effect weapon in the torpedo bay with blast damage, a reworked per-section health system, and sharper directional and thruster shaders.",
    },
    {
        slug: "devlog-2-objectives-enemy-ai-and-asteroids",
        title: "Devlog #2: objectives, enemy AI and better asteroids",
        date: "2025-11-08",
        version: "v0.2.0",
        description:
            "Nova Protocol v0.2.0: a data-driven events/filters/actions modding system for objectives, the first (gloriously dumb) enemy AI, procedurally generated asteroids, and a physics fight with GlobalTransform.",
    },
    {
        slug: "devlog-1-modular-ships-and-first-combat",
        title: "Devlog #1: modular ships and first combat",
        date: "2025-10-21",
        version: "v0.1.0",
        description:
            "How Nova Protocol v0.1.0 came together: thruster-driven modular ships, a PD-controlled mouse steering section, turrets that shoot, and a health system that blows sections into chunks.",
    },
];
const postPage = (p) =>
    blogPostPage({ ...p, mdPath: `src/posts/${p.slug}.md`, publicPath });

const config = {
    entry: {
        index: "./src/index.ts",
        blog: "./src/blog.ts",
        post: "./src/post.ts",
        tutorial: "./src/tutorial.ts",
        wiki: "./src/wiki.ts",
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
        page("blog", "src/blog.html", "blog/index.html"),
        ...BLOG_POSTS.map(postPage),
        page("tutorial", "src/tutorial.html", "tutorial/index.html"),
        page("wiki", "src/wiki.html", "wiki/index.html"),
        ...WIKI_DOC_PAGES.map(docPage),
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
                ...BLOG_POSTS.map(({ slug }) => ({
                    from: new RegExp("^/blog/" + slug),
                    to: "/blog/" + slug + "/index.html",
                })),
                { from: /^\/blog/, to: "/blog/index.html" },
                { from: /^\/tutorial/, to: "/tutorial/index.html" },
                ...WIKI_DOC_PAGES.map(({ slug }) => ({
                    from: new RegExp("^/wiki/" + slug),
                    to: "/wiki/" + slug + "/index.html",
                })),
                { from: /^\/wiki/, to: "/wiki/index.html" },
            ],
        },
    },
    watchOptions: {
        ignored: ["**/node_modules/**", "**/dist/**"],
    },
};

module.exports = config;
