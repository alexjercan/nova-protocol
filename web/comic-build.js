const HtmlWebpackPlugin = require("html-webpack-plugin");

function escapeHtml(value) {
    return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}
function episodeUrl(comic, episode, publicPath) {
    return `${publicPath}story/${comic.path}/${episode.id}/`;
}

/** Released reading order never includes local draft episodes. */
function releasedEpisodes(comics) {
    return comics
        .slice()
        .sort((a, b) => a.sequence - b.sequence)
        .flatMap((comic) =>
            comic.episodes
                .filter((e) => e.publication === "released")
                .map((episode) => ({ comic, episode }))
        );
}
function htmlPage(filename, title, summary, body, publicPath, reader = false) {
    return new HtmlWebpackPlugin({
        filename,
        chunks: ["story"],
        basePath: publicPath,
        templateContent: `<!doctype html><html lang="en"><head><meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>${escapeHtml(title)} - Nova Protocol</title><meta name="description" content="${escapeHtml(summary)}" />
<link rel="icon" href="${publicPath}story/favicon.svg" /></head><body${reader ? ' class="comic-body"' : ""}>
${reader ? "" : '<div id="header"></div>'}${body}${reader ? "" : '<div id="footer"></div>'}</body></html>`,
    });
}
function countLabel(count, noun) {
    return `${count} ${noun}${count === 1 ? "" : "s"}`;
}
function isDraft(episode) {
    return episode.publication === "draft";
}
function seasonUrl(comic, publicPath) {
    return `${publicPath}story/${comic.path}/`;
}
function episodeList(comic, publicPath) {
    return `<ol class="story-episodes">${comic.episodes
        .map(
            (
                episode
            ) => `<li><a class="story-episode" href="${episodeUrl(comic, episode, publicPath)}">
<img src="${publicPath}story/assets/${comic.path}/${escapeHtml(episode.cover)}" alt="${escapeHtml(episode.coverAlt)}" loading="lazy" />
<span class="story-row__body"><span class="story-meta">Episode ${String(episode.number).padStart(2, "0")}</span>
<strong>${escapeHtml(episode.title)}</strong><span class="story-meta">${isDraft(episode) ? `<span class="story-draft-label">Draft</span> - ${episode.pages.length} of ${episode.pageCount} pages illustrated` : countLabel(episode.pages.length, "page")}</span><span>${escapeHtml(episode.summary)}</span></span></a></li>`
        )
        .join("")}</ol>`;
}
function seasonRow(comic, publicPath) {
    const drafts = comic.episodes.filter(isDraft).length;
    return `<li><a class="story-season" href="${seasonUrl(comic, publicPath)}">
<img src="${publicPath}story/assets/${comic.path}/${escapeHtml(comic.cover)}" alt="${escapeHtml(comic.coverAlt)}" loading="lazy" />
<span class="story-row__body"><strong>${escapeHtml(comic.title)}</strong>
<span class="story-meta">${countLabel(comic.episodes.length, "episode")}${drafts ? ` / <span class="story-draft-label">${countLabel(drafts, "draft")}</span>` : ""}</span>
<span>${escapeHtml(comic.summary)}</span></span></a></li>`;
}

/** List selected seasons without repeating their episode lists. */
function comicIndexPage(comics, publicPath) {
    const seasons = comics.slice().sort((a, b) => a.sequence - b.sequence);
    return htmlPage(
        "story/index.html",
        "Story",
        "Read Nova Protocol by season and episode.",
        `<main><section class="section story-index"><div class="container">
<h1 class="section__title">Story</h1><p class="section__lead">The Nova Protocol comic. Explore the world in the <a href="${publicPath}lore/">lore encyclopedia</a>.</p>
${seasons.length ? `<ol class="story-seasons">${seasons.map((comic) => seasonRow(comic, publicPath)).join("")}</ol>` : '<p class="story-index__empty">No story episodes have been released yet.</p>'}
</div></section></main>`,
        publicPath
    );
}

/** List a selected season's episodes once, in authored reading order. */
function comicSeasonPage(comic, publicPath) {
    return htmlPage(
        `story/${comic.path}/index.html`,
        comic.title,
        comic.summary,
        `<main><section class="section story-index story-season-page"><div class="container">
<a class="story-back" href="${publicPath}story/">Story</a>
<h1 class="section__title">${escapeHtml(comic.title)}</h1>
<p class="section__lead">${escapeHtml(comic.summary)}</p>
${episodeList(comic, publicPath)}</div></section></main>`,
        publicPath
    );
}

/** Render one episode using the same player for development and publication. */
function comicReaderPage(comic, episode, publicPath, comics = [comic]) {
    const draft = isDraft(episode);
    const order = draft
        ? comics
              .slice()
              .sort((a, b) => a.sequence - b.sequence)
              .flatMap((comic) =>
                  comic.episodes.map((episode) => ({ comic, episode }))
              )
        : releasedEpisodes(comics);
    const current = order.findIndex(
        (e) => e.comic.path === comic.path && e.episode.id === episode.id
    );
    if (current < 0) throw new Error("Episode is not in the reading order");
    const adjacent = (entry, label) =>
        entry
            ? `<a href="${episodeUrl(entry.comic, entry.episode, publicPath)}">${label}: ${escapeHtml(entry.episode.title)}</a>`
            : "";
    const contents = episode.pages
        .map(
            (page, index) =>
                `<a href="#${escapeHtml(page.id)}" data-page-link="${escapeHtml(page.id)}"><span>${String(index + 1).padStart(2, "0")}</span>${escapeHtml(page.title)}</a>`
        )
        .join("");
    const definition = JSON.stringify({
        path: comic.path,
        title: comic.title,
        episodeTitle: episode.title,
        pages: episode.pages,
        basePath: publicPath,
    }).replace(/</g, "\\u003c");
    return htmlPage(
        `story/${comic.path}/${episode.id}/index.html`,
        episode.title,
        episode.summary,
        `<main class="comic-reader" data-comic-reader data-contents-open="false"><header class="comic-reader__toolbar">
<div class="comic-reader__identity">${draft ? '<span class="story-draft-label">Local draft</span>' : ""}<a class="comic-reader__back" href="${seasonUrl(comic, publicPath)}">${escapeHtml(comic.title)}</a><strong>${escapeHtml(episode.title)}</strong></div>
<div class="comic-reader__tools">${draft ? '<button class="comic-reader__tool" data-art-toggle aria-pressed="false">Art only</button>' : ""}<a data-page-art hidden target="_blank" rel="noopener">Open image</a><button class="comic-reader__tool" type="button" data-contents-toggle aria-controls="comic-contents" aria-expanded="false">Pages</button><button class="comic-reader__tool comic-reader__tool--compact" type="button" data-reader-dialog="panels" aria-controls="comic-panels" aria-haspopup="dialog" hidden>Panels</button><button class="comic-reader__tool comic-reader__tool--compact" type="button" data-reader-dialog="transcript" aria-controls="comic-transcript" aria-haspopup="dialog">Transcript</button><span class="comic-reader__counter" aria-live="polite">Page <b data-page-current>01</b> / ${episode.pages.length}</span></div></header>
<div class="comic-reader__body"><aside class="comic-reader__contents" id="comic-contents" aria-label="Episode pages" hidden><p class="comic-reader__contents-title">${escapeHtml(comic.title)} / ${escapeHtml(episode.title)}</p><nav>${contents}</nav></aside>
<div class="comic-reader__viewport" data-page-viewport tabindex="0" aria-label="${escapeHtml(episode.title)} comic pages"></div></div>
<footer class="comic-reader__pager" aria-label="Page and episode navigation"><button type="button" data-page-previous disabled>Previous page</button><div class="story-episode-links">${adjacent(order[current - 1], "Previous episode")}${adjacent(order[current + 1], "Next episode")}</div><button type="button" data-page-next>Next page</button></footer>
<dialog class="comic-reader-dialog" id="comic-panels" aria-labelledby="comic-panels-title"><header><h2 id="comic-panels-title">Panels</h2><button class="comic-reader__tool" type="button" data-dialog-close>Close</button></header><p class="comic-reader-dialog__page" data-dialog-page></p><div class="comic-reader-dialog__body" tabindex="0"><ul data-panel-image-list></ul></div></dialog>
<dialog class="comic-reader-dialog" id="comic-transcript" aria-labelledby="comic-transcript-title"><header><h2 id="comic-transcript-title">Transcript</h2><button class="comic-reader__tool" type="button" data-dialog-close>Close</button></header><p class="comic-reader-dialog__page" data-dialog-page></p><div class="comic-reader-dialog__body" tabindex="0"><p data-page-transcript></p></div></dialog></main>
<noscript><style>.comic-body{height:auto;overflow:auto}.comic-reader{display:none}noscript .story-page-text{white-space:pre-wrap}</style><p>This reader needs JavaScript. The page text is also available below.</p>${episode.pages.map((page) => `<h2>${escapeHtml(page.title)}</h2><p class="story-page-text">${escapeHtml(page.transcript)}</p>`).join("")}</noscript>
<script id="comic-definition" type="application/json">${definition}</script>`,
        publicPath,
        true
    );
}

/** Exact episode routes precede their season route in development fallbacks. */
function comicRoutes(comics) {
    return comics.flatMap((comic) => [
        ...comic.episodes.map((episode) => `story/${comic.path}/${episode.id}`),
        `story/${comic.path}`,
    ]);
}
module.exports = {
    comicIndexPage,
    comicSeasonPage,
    comicReaderPage,
    comicRoutes,
    releasedEpisodes,
};
