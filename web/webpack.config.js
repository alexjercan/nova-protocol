const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const HtmlPartialsPlugin = require("./webpack-partials");
const CopyPlugin = require("copy-webpack-plugin");

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
        page(
            "post",
            "src/posts/building-nova-protocol.html",
            "blog/building-nova-protocol/index.html"
        ),
        page(
            "post",
            "src/posts/devlog-5-radar-locking-shakedown-and-the-web.html",
            "blog/devlog-5-radar-locking-shakedown-and-the-web/index.html"
        ),
        page(
            "post",
            "src/posts/devlog-4-guided-torpedoes-targeting-and-enemy-ai.html",
            "blog/devlog-4-guided-torpedoes-targeting-and-enemy-ai/index.html"
        ),
        page(
            "post",
            "src/posts/devlog-3-zones-torpedoes-and-blast-damage.html",
            "blog/devlog-3-zones-torpedoes-and-blast-damage/index.html"
        ),
        page(
            "post",
            "src/posts/devlog-2-objectives-enemy-ai-and-asteroids.html",
            "blog/devlog-2-objectives-enemy-ai-and-asteroids/index.html"
        ),
        page(
            "post",
            "src/posts/devlog-1-modular-ships-and-first-combat.html",
            "blog/devlog-1-modular-ships-and-first-combat/index.html"
        ),
        page("tutorial", "src/tutorial.html", "tutorial/index.html"),
        page("wiki", "src/wiki.html", "wiki/index.html"),
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
                {
                    from: /^\/blog\/building-nova-protocol/,
                    to: "/blog/building-nova-protocol/index.html",
                },
                {
                    from: /^\/blog\/devlog-5-radar-locking-shakedown-and-the-web/,
                    to: "/blog/devlog-5-radar-locking-shakedown-and-the-web/index.html",
                },
                {
                    from: /^\/blog\/devlog-4-guided-torpedoes-targeting-and-enemy-ai/,
                    to: "/blog/devlog-4-guided-torpedoes-targeting-and-enemy-ai/index.html",
                },
                {
                    from: /^\/blog\/devlog-3-zones-torpedoes-and-blast-damage/,
                    to: "/blog/devlog-3-zones-torpedoes-and-blast-damage/index.html",
                },
                {
                    from: /^\/blog\/devlog-2-objectives-enemy-ai-and-asteroids/,
                    to: "/blog/devlog-2-objectives-enemy-ai-and-asteroids/index.html",
                },
                {
                    from: /^\/blog\/devlog-1-modular-ships-and-first-combat/,
                    to: "/blog/devlog-1-modular-ships-and-first-combat/index.html",
                },
                { from: /^\/blog/, to: "/blog/index.html" },
                { from: /^\/tutorial/, to: "/tutorial/index.html" },
                { from: /^\/wiki/, to: "/wiki/index.html" },
            ],
        },
    },
    watchOptions: {
        ignored: ["**/node_modules/**", "**/dist/**"],
    },
};

module.exports = config;
