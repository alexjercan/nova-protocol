const fs = require("fs");
const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");

const CHAPTER_STATES = ["playable", "planned", "frame"];
const DEFAULT_ROOTS = {
    comics: path.resolve(__dirname, "src/comics"),
    assets: path.resolve(__dirname, "src/assets/story"),
};

function escapeHtml(value) {
    return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

/**
 * Read every `<dir>/comic.json` under the comics root and validate it. Loose
 * files beside the comic directories are the engine, not comics; a root with
 * no directories is an empty archive.
 */
function discoverComics(roots = DEFAULT_ROOTS) {
    if (!fs.existsSync(roots.comics)) return [];
    return fs
        .readdirSync(roots.comics, { withFileTypes: true })
        .filter((entry) => entry.isDirectory())
        .map((entry) => loadComic(entry.name, roots))
        .sort((a, b) => a.title.localeCompare(b.title));
}

function loadComic(comicPath, roots = DEFAULT_ROOTS) {
    const manifestPath = path.join(roots.comics, comicPath, "comic.json");
    if (!fs.existsSync(manifestPath)) {
        throw new Error(`comic '${comicPath}' has no comic.json`);
    }
    const comic = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    validateComic(comicPath, comic, roots);
    return { ...comic, path: comicPath };
}

function validateComic(comicPath, comic, roots = DEFAULT_ROOTS) {
    // Must stay in step with `comic-catalog.ts`'s require.context, which
    // matches `<comic>/pages/<name>.ts` and nothing deeper: a source the
    // build accepts but the loader cannot resolve ships an empty reader.
    const pathPattern = /^pages\/[a-z0-9][a-z0-9-]*$/;
    const idPattern = /^[a-z0-9][a-z0-9-]*$/;
    const isId = (value) => typeof value === "string" && idPattern.test(value);
    if (!isId(comicPath)) {
        throw new Error(`comic directory '${comicPath}' is not a valid id`);
    }
    for (const field of ["title", "summary", "status", "cover", "coverAlt"]) {
        if (typeof comic[field] !== "string" || !comic[field].trim()) {
            throw new Error(`comic '${comicPath}' has no ${field}`);
        }
    }
    if (!Array.isArray(comic.chapters) || !comic.chapters.length) {
        throw new Error(`comic '${comicPath}' has no chapters`);
    }
    const chapterIds = new Set();
    const pageIds = new Set();
    for (const chapter of comic.chapters) {
        if (!isId(chapter.id) || chapterIds.has(chapter.id)) {
            throw new Error(
                `comic '${comicPath}' has an invalid/duplicate chapter id`
            );
        }
        chapterIds.add(chapter.id);
        if (!CHAPTER_STATES.includes(chapter.state)) {
            throw new Error(
                `comic '${comicPath}' chapter '${chapter.id}' needs a state: ${CHAPTER_STATES.join(", ")}`
            );
        }
        if (
            !chapter.title ||
            !Array.isArray(chapter.pages) ||
            !chapter.pages.length
        ) {
            throw new Error(
                `comic '${comicPath}' chapter '${chapter.id}' is incomplete`
            );
        }
        for (const page of chapter.pages) {
            if (!isId(page.id) || pageIds.has(page.id)) {
                throw new Error(
                    `comic '${comicPath}' has an invalid/duplicate page id`
                );
            }
            pageIds.add(page.id);
            if (!page.title || !pathPattern.test(page.source)) {
                throw new Error(
                    `comic '${comicPath}' page '${page.id}' is incomplete or has an invalid source`
                );
            }
            const source = path.join(
                roots.comics,
                comicPath,
                `${page.source}.ts`
            );
            if (!fs.existsSync(source)) {
                throw new Error(
                    `comic '${comicPath}' page source is missing: ${source}`
                );
            }
        }
    }
    assertAsset(comicPath, comic.cover, roots, "cover");
    for (const module of comicModules(path.join(roots.comics, comicPath))) {
        for (const asset of referencedAssets(module)) {
            assertAsset(comicPath, asset, roots, path.basename(module));
        }
    }
}

/** Every TypeScript module under a comic directory. */
function comicModules(directory) {
    return fs
        .readdirSync(directory, { withFileTypes: true })
        .flatMap((entry) => {
            const full = path.join(directory, entry.name);
            if (entry.isDirectory()) return comicModules(full);
            return entry.name.endsWith(".ts") ? [full] : [];
        });
}

/** Asset file names a page module mentions as string literals. */
function referencedAssets(modulePath) {
    const text = fs.readFileSync(modulePath, "utf8");
    const found = new Set();
    for (const match of text.matchAll(
        /"([a-z0-9][a-z0-9/-]*\.(?:svg|png|webp))"/g
    )) {
        found.add(match[1]);
    }
    return [...found];
}

function assertAsset(comicPath, source, roots, owner) {
    const assetRoot = path.resolve(roots.assets, comicPath);
    const file = path.resolve(assetRoot, source);
    if (!file.startsWith(`${assetRoot}${path.sep}`) || !fs.existsSync(file)) {
        throw new Error(
            `comic '${comicPath}' ${owner} asset is missing: ${source}`
        );
    }
}

function pagesOf(comic) {
    return comic.chapters.flatMap((chapter) =>
        chapter.pages.map((page) => ({
            ...page,
            chapter: chapter.title,
            chapterId: chapter.id,
            state: chapter.state,
        }))
    );
}

/** How many chapters are playable in the game and how many are only planned. */
function chapterCounts(comic) {
    const counts = { playable: 0, planned: 0, frame: 0 };
    for (const chapter of comic.chapters) counts[chapter.state] += 1;
    return counts;
}

function comicIndexPage(comics, publicPath) {
    const cards = comics
        .map((comic) => {
            const pages = pagesOf(comic);
            const counts = chapterCounts(comic);
            return `<li class="post-card story-record">
                <a class="post-card__link" href="${publicPath}story/${escapeHtml(comic.path)}/">
                    <div class="post-card__media">
                        <img class="post-card__thumb" src="${publicPath}assets/story/${escapeHtml(comic.path)}/${escapeHtml(comic.cover)}" alt="${escapeHtml(comic.coverAlt)}" />
                        <span class="story-record__status">${escapeHtml(comic.status)}</span>
                    </div>
                    <div class="post-card__body">
                        <span class="post-card__meta">${counts.playable} playable // ${counts.planned} planned // ${pages.length} pages</span>
                        <h2 class="post-card__title">${escapeHtml(comic.title)}</h2>
                        <p class="post-card__excerpt">${escapeHtml(comic.summary)}</p>
                        <span class="story-record__open">Open comic [ENTER]</span>
                    </div>
                </a>
            </li>`;
        })
        .join("\n");
    const records = comics.length
        ? `<ul class="post-grid story-index__grid">${cards}</ul>`
        : `<p class="story-index__empty">No campaign records yet.</p>`;
    return new HtmlWebpackPlugin({
        filename: "story/index.html",
        chunks: ["story"],
        basePath: publicPath,
        templateContent: `<!doctype html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Stories - Nova Protocol</title>
    <meta name="description" content="Read Nova Protocol campaigns as digital HUD comics." />
    <link rel="icon" href="${publicPath}favicon.svg" />
</head>
<body>
    <div id="header"></div>
    <main>
        <section class="section story-index">
            <div class="container">
                <p class="section__eyebrow">Story archive</p>
                <h1 class="section__title">Campaign records //<br /><span class="glow-phosphor">select a story</span></h1>
                <p class="section__lead">Each campaign has its own digital comic. Open a record to read it page by page or jump from its contents display. Planned chapters are marked: they are the story's draft, not yet a mission you can fly. These records contain full campaign spoilers.</p>
                ${records}
            </div>
        </section>
    </main>
    <div id="footer"></div>
</body>
</html>`,
    });
}

function comicReaderPage(comic, publicPath) {
    const pages = pagesOf(comic);
    const counts = chapterCounts(comic);
    let number = 0;
    const contents = comic.chapters
        .map(
            (chapter) => `<p>${escapeHtml(chapter.title)}${
                chapter.state === "planned"
                    ? ' <em class="comic-reader__planned">planned</em>'
                    : ""
            }</p>
                ${chapter.pages
                    .map((page) => {
                        number += 1;
                        return `<a href="#${escapeHtml(page.id)}" data-page-link="${escapeHtml(page.id)}"><span>${String(number).padStart(2, "0")}</span> ${escapeHtml(page.title)}</a>`;
                    })
                    .join("\n")}`
        )
        .join("\n");
    const definition = JSON.stringify({
        ...comic,
        pages,
        basePath: publicPath,
    }).replace(/</g, "\\u003c");
    return new HtmlWebpackPlugin({
        filename: `story/${comic.path}/index.html`,
        chunks: ["story"],
        basePath: publicPath,
        templateContent: `<!doctype html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${escapeHtml(comic.title)} - Story</title>
    <meta name="description" content="${escapeHtml(comic.summary)}" />
    <link rel="icon" href="${publicPath}favicon.svg" />
</head>
<body class="comic-body">
    <main class="comic-reader" data-comic-reader data-contents-open="true">
        <header class="comic-reader__toolbar">
            <div class="comic-reader__identity">
                <a href="${publicPath}story/" class="comic-reader__back">Exit campaign</a>
                <span class="comic-reader__divider">//</span>
                <strong>${escapeHtml(comic.title)}</strong>
            </div>
            <div class="comic-reader__tools">
                <button class="comic-reader__tool" type="button" data-contents-toggle aria-controls="comic-contents" aria-expanded="true">Contents</button>
                <span class="comic-reader__counter" aria-live="polite">Page <b data-page-current>01</b> / ${String(pages.length).padStart(2, "0")}</span>
            </div>
        </header>
        <div class="comic-reader__body">
            <aside class="comic-reader__contents" id="comic-contents" aria-label="Comic contents">
                <p class="comic-reader__contents-title">Contents</p>
                <nav>${contents}</nav>
                <div class="comic-reader__legend"><span>Campaign status</span><strong>${escapeHtml(comic.status)}</strong><span>Playable now</span><strong>${counts.playable} chapters</strong><span>Planned</span><strong>${counts.planned} chapters</strong></div>
            </aside>
            <div class="comic-reader__viewport" data-page-viewport tabindex="0" aria-label="${escapeHtml(comic.title)} comic pages"></div>
        </div>
        <footer class="comic-reader__pager" aria-label="Page navigation">
            <button type="button" data-page-previous disabled>Previous page</button>
            <div class="comic-reader__progress" aria-hidden="true"><i data-page-progress></i></div>
            <button type="button" data-page-next>Next page</button>
        </footer>
    </main>
    <script id="comic-definition" type="application/json">${definition}</script>
</body>
</html>`,
    });
}

module.exports = {
    CHAPTER_STATES,
    chapterCounts,
    comicIndexPage,
    comicReaderPage,
    discoverComics,
    loadComic,
    pagesOf,
    referencedAssets,
    validateComic,
};
