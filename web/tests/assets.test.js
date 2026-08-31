// The news namespace gate (plain node, no tsc: this reads source files off
// disk and has no DOM dependency).
//
// One rule, both directions:
//
//   A `news-*` image or loop belongs to exactly one post. Only that post may
//   show it, and a post may show nothing else.
//
// The forward direction keeps a shipped post honest. A post argues about one
// release; a figure named for a living asset is re-cut every cycle, so the
// post silently ends up illustrated with footage of a game it was never
// about. The packagers freeze the `news-` namespace against exactly that
// (`frozen` in scripts/gen-web-screenshots.py, `is_frozen` in
// scripts/capture-web-media.sh) - this gate is what makes the freeze cover
// the whole post rather than the figures someone remembered to rename.
//
// The reverse direction keeps the living pages current. The landing page and
// the wiki show what the game does NOW, so a `news-` figure on one of them is
// frozen at an old release and can never update. That is not hypothetical:
// the landing page's damage feature row pointed at `news-0110-release-lead`
// and was pinned to v0.11.0 until it got a scene of its own.

const assert = require("assert");
const fs = require("fs");
const path = require("path");

const SRC = path.join(__dirname, "..", "src");

// A reference to a shipped still or loop. Prose naming a game asset path
// (`assets/base/`, `assets/mods.catalog.ron`) is not a figure, so the
// extension is what selects.
const REFERENCE = /assets\/(?:loops\/)?([A-Za-z0-9._-]+\.(?:png|webm))/g;

const SCANNED = new Set([".html", ".md", ".ts", ".js"]);

/** Every scannable source file under `dir`, recursively, skipping assets/. */
function sources(dir) {
    const out = [];
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            if (entry.name === "assets") continue;
            out.push(...sources(full));
        } else if (SCANNED.has(path.extname(entry.name))) {
            out.push(full);
        }
    }
    return out;
}

/** Every distinct still/loop file name referenced by `file`. */
function references(file) {
    const text = fs.readFileSync(file, "utf8");
    const names = new Set();
    for (const match of text.matchAll(REFERENCE)) names.add(match[1]);
    return [...names];
}

const isNews = (name) => name.startsWith("news-");
const inNewsPost = (file) =>
    path.dirname(file) === path.join(SRC, "news") && file.endsWith(".md");

const strays = [];
const borrowed = [];

for (const file of sources(SRC)) {
    const where = path.relative(SRC, file);
    for (const name of references(file)) {
        if (inNewsPost(file)) {
            if (!isNews(name)) strays.push(`${where} -> ${name}`);
        } else if (isNews(name)) {
            borrowed.push(`${where} -> ${name}`);
        }
    }
}

assert.deepStrictEqual(
    strays,
    [],
    "a news post may only show `news-*` figures; these name a living asset " +
        "that the next capture run re-cuts:\n  " +
        strays.join("\n  ") +
        "\nalias each one into the post's namespace in " +
        "scripts/gen-web-screenshots.py or scripts/capture-web-media.sh."
);

assert.deepStrictEqual(
    borrowed,
    [],
    "only a news post may show a `news-*` figure; these pages are pinned to " +
        "an old release and can never update:\n  " +
        borrowed.join("\n  ") +
        "\ngive each one a living name with a producer of its own."
);

// The rule is worth nothing if the pattern silently stops matching the markup,
// so pin it against the two shapes the site actually writes.
assert.deepStrictEqual(
    [
        ...'<span class="figure__placeholder-name">assets/news-0120-drives.png</span>'.matchAll(
            REFERENCE
        ),
    ].map((m) => m[1]),
    ["news-0120-drives.png"]
);
assert.deepStrictEqual(
    [
        ..."<!-- Capture: assets/loops/landing-cockpit.webm -->".matchAll(
            REFERENCE
        ),
    ].map((m) => m[1]),
    ["landing-cockpit.webm"]
);
assert.deepStrictEqual(
    [..."see assets/base/sounds/ for the layout".matchAll(REFERENCE)],
    []
);

console.log(
    `assets: news namespace held across ${sources(SRC).length} source files`
);
