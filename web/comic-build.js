const fs = require("fs");
const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");

function escapeHtml(value) {
    return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

function discoverComics() {
    const root = path.resolve(__dirname, "src/comics");
    if (!fs.existsSync(root)) return [];
    return fs
        .readdirSync(root, { withFileTypes: true })
        .filter((entry) => entry.isDirectory())
        .map((entry) => {
            const comicPath = entry.name;
            const manifestPath = path.join(root, comicPath, "comic.json");
            if (!fs.existsSync(manifestPath)) {
                throw new Error(`comic '${comicPath}' has no comic.json`);
            }
            const comic = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
            validateComic(comicPath, comic, root);
            return { ...comic, path: comicPath };
        })
        .sort((a, b) => a.title.localeCompare(b.title));
}

function validateComic(comicPath, comic, root) {
    const pathPattern = /^[a-z0-9][a-z0-9/-]*$/;
    const idPattern = /^[a-z0-9][a-z0-9-]*$/;
    for (const field of ["title", "summary", "status", "cover"]) {
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
        if (!idPattern.test(chapter.id) || chapterIds.has(chapter.id)) {
            throw new Error(
                `comic '${comicPath}' has an invalid/duplicate chapter id`
            );
        }
        chapterIds.add(chapter.id);
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
            if (!idPattern.test(page.id) || pageIds.has(page.id)) {
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
            const source = path.join(root, comicPath, `${page.source}.ts`);
            if (!fs.existsSync(source)) {
                throw new Error(
                    `comic '${comicPath}' page source is missing: ${source}`
                );
            }
        }
    }
    const assetRoot = path.resolve(__dirname, "src/assets/story", comicPath);
    const cover = path.resolve(assetRoot, comic.cover);
    if (!cover.startsWith(`${assetRoot}${path.sep}`) || !fs.existsSync(cover)) {
        throw new Error(`comic '${comicPath}' cover is missing: ${cover}`);
    }
}

function pagesOf(comic) {
    return comic.chapters.flatMap((chapter) =>
        chapter.pages.map((page) => ({ ...page, chapter: chapter.title }))
    );
}

function comicIndexPage(comics, publicPath) {
    const cards = comics
        .map((comic) => {
            const pages = pagesOf(comic);
            return `<li class="post-card story-record">
                <a class="post-card__link" href="${publicPath}story/${escapeHtml(comic.path)}/">
                    <div class="post-card__media">
                        <img class="post-card__thumb" src="${publicPath}assets/story/${escapeHtml(comic.path)}/${escapeHtml(comic.cover)}" alt="${escapeHtml(comic.coverAlt || comic.title)}" />
                        <span class="story-record__status">${escapeHtml(comic.status)}</span>
                    </div>
                    <div class="post-card__body">
                        <span class="post-card__meta">${comic.chapters.length} chapters // ${pages.length} pages</span>
                        <h2 class="post-card__title">${escapeHtml(comic.title)}</h2>
                        <p class="post-card__excerpt">${escapeHtml(comic.summary)}</p>
                        <span class="story-record__open">Open comic [ENTER]</span>
                    </div>
                </a>
            </li>`;
        })
        .join("\n");
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
                <p class="section__lead">Each campaign has its own digital comic. Open a record to read it page by page or jump from its contents display. These records contain full campaign spoilers.</p>
                <ul class="post-grid story-index__grid">${cards}</ul>
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
    let number = 0;
    const contents = comic.chapters
        .map(
            (chapter) => `<p>${escapeHtml(chapter.title)}</p>
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
                <div class="comic-reader__legend"><span>Campaign status</span><strong>${escapeHtml(comic.status)}</strong><span>Current release</span><strong>${comic.chapters.length} playable chapters</strong></div>
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

module.exports = { discoverComics, comicIndexPage, comicReaderPage, pagesOf };
