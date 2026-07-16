const fs = require("fs");
const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");

// markdown-it and its plugins ship dual CJS/ESM; require() may hand back either
// the value or a { default } wrapper, so normalise.
const MarkdownIt = interop(require("markdown-it"));
const anchor = interop(require("markdown-it-anchor"));
const hljs = interop(require("highlight.js"));

function interop(m) {
    return m && m.__esModule && m.default ? m.default : m;
}

// Build-time markdown -> HTML for the developer wiki pages. Rendering happens
// here in Node (the webpack config calls wikiDocPage at configure time), so
// there is no runtime markdown cost and a no-JS / SEO reader still gets the full
// article - the same "content in HTML, chrome via JS" split the hand-authored
// wiki pages use.
//
// - A fenced ```mermaid block becomes a <pre class="mermaid"> holding the escaped
//   diagram source; wiki.ts renders it client-side (mermaid needs the DOM). Every
//   other fence is highlighted with highlight.js into <pre><code class="hljs ...">.
// - markdown-it-anchor gives every h2/h3 a slug id, so headings deep-link and the
//   manifest's `headings` search terms line up with real anchors.
// - html: true passes raw inline HTML through untouched - the escape hatch for a
//   custom widget or embed inside a doc.
const md = new MarkdownIt({
    html: true,
    linkify: true,
    typographer: false,
    highlight(code, lang) {
        if (lang === "mermaid") {
            return `<pre class="mermaid">${md.utils.escapeHtml(code)}</pre>`;
        }
        const language = lang && hljs.getLanguage(lang) ? lang : null;
        const body = language
            ? hljs.highlight(code, { language, ignoreIllegals: true }).value
            : md.utils.escapeHtml(code);
        const cls = language ? ` language-${language}` : "";
        // Returning a string that starts with "<pre" tells markdown-it's fence
        // renderer to emit it verbatim (no extra <pre><code> wrapping).
        return `<pre><code class="hljs${cls}">${body}</code></pre>`;
    },
});

md.use(anchor, {
    level: [2, 3],
    slugify: (s) =>
        s
            .toLowerCase()
            .trim()
            .replace(/[^\w\s-]/g, "")
            .replace(/\s+/g, "-"),
});

// Render a markdown file to { html, title, headings }. The leading H1 is pulled
// out and returned as the title (the doc shell renders it, so the body starts at
// the first real section) - keeping the crumb/h1/tags order identical to the
// hand-authored wiki pages. `headings` is the list of h2/h3 sections (with the
// markdown-it-anchor slug id and plain text), used to build the news TOC
// sidebar; other shells ignore it.
function renderMarkdownFile(mdPath) {
    const src = fs.readFileSync(mdPath, "utf8");
    const env = {};
    const tokens = md.parse(src, env);

    let title = "";
    const i = tokens.findIndex(
        (t) => t.type === "heading_open" && t.tag === "h1"
    );
    if (i !== -1) {
        const inline = tokens[i + 1];
        if (inline && inline.type === "inline") title = inline.content;
        tokens.splice(i, 3); // heading_open, inline, heading_close
    }

    // Collect h2/h3 headings for a table of contents. markdown-it-anchor has
    // already set each heading_open token's `id` attr (it runs during parse),
    // so the ids line up with the real in-page anchors. Strip inline markdown
    // markers from the display text (headings are plain, but be safe).
    const headings = [];
    for (let j = 0; j < tokens.length; j++) {
        const t = tokens[j];
        if (t.type === "heading_open" && (t.tag === "h2" || t.tag === "h3")) {
            const inline = tokens[j + 1];
            const raw =
                inline && inline.type === "inline" ? inline.content : "";
            const text = raw.replace(/[*_`]/g, "").trim();
            const id = t.attrGet("id");
            if (id && text) headings.push({ level: t.tag, id, text });
        }
    }

    const html = md.renderer.render(tokens, md.options, env);
    return { html, title, headings };
}

function escapeAttr(s) {
    return String(s)
        .replace(/&/g, "&amp;")
        .replace(/"/g, "&quot;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
}

// The page shell for a markdown doc: the same chrome as a hand-authored wiki
// page (header/footer placeholders, the manifest-driven #wiki-nav aside, the
// crumb/h1/#wiki-tags, and #wiki-seealso), with a #doc-body placeholder the
// partials plugin fills with the rendered markdown after templating - so lodash
// never runs over code samples. Unlike a `template` FILE, a templateContent
// STRING is not run through lodash, so basePath is inlined here at config time
// (publicPath is already known) rather than left as a <%= %> token.
// opts: { description, crumbParent: { slug, title } }. A description is rendered
// as the page meta; a crumbParent renders a two-level crumb ("Wiki / <parent> /
// <title>") for child pages like the ship sections.
function docShell(title, basePath, opts = {}) {
    const t = escapeAttr(title);
    const b = escapeAttr(basePath);
    const desc = opts.description
        ? `\n        <meta name="description" content="${escapeAttr(
              opts.description
          )}" />`
        : "";
    const parent = opts.crumbParent;
    const crumb = parent
        ? `<a href="${b}wiki/">Wiki</a>
                        / <a href="${b}wiki/${escapeAttr(parent.slug)}/">${escapeAttr(
                            parent.title
                        )}</a>
                        / ${t}`
        : `<a href="${b}wiki/">Wiki</a>
                        / ${t}`;
    return `<!doctype html>
<html lang="en">
    <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>${t} - Nova Protocol Wiki</title>${desc}
        <link rel="icon" href="${b}favicon.svg" />
    </head>
    <body>
        <div id="header"></div>
        <main>
            <div class="wiki">
                <aside
                    class="wiki__nav"
                    id="wiki-nav"
                    aria-label="Wiki navigation"
                ></aside>
                <article class="wiki__body prose">
                    <p class="wiki__crumb">
                        ${crumb}
                    </p>
                    <h1>${t}</h1>
                    <div class="wiki__tags" id="wiki-tags"></div>
                    <div id="doc-body"></div>
                    <div id="wiki-seealso"></div>
                </article>
            </div>
        </main>
        <div id="footer"></div>
    </body>
</html>`;
}

// Build one HtmlWebpackPlugin for a markdown doc page. The rendered body rides
// on the plugin's `docBody` option; HtmlPartialsPlugin injects it into the
// #doc-body placeholder at beforeEmit (see webpack-partials.js). Shares the
// `wiki` chunk so the sidebar/search/tags/see-also all render from the manifest.
// `description` sets the page meta; `crumbParent` renders a child crumb.
function wikiDocPage({
    slug,
    mdPath,
    title,
    description,
    crumbParent,
    publicPath,
}) {
    const abs = path.resolve(__dirname, mdPath);
    const { html, title: h1 } = renderMarkdownFile(abs);
    const pageTitle = title || h1;
    return new HtmlWebpackPlugin({
        filename: `wiki/${slug}/index.html`,
        chunks: ["wiki"],
        basePath: publicPath,
        docBody: html,
        templateContent: docShell(pageTitle, publicPath, {
            description,
            crumbParent,
        }),
    });
}

// The page shell for a NEWS post - the merged devlog + release-notes page. One
// post per feature release: a standalone `.prose` article carrying the
// narrative intro, the structured "what's new" highlights, any breaking-changes
// callout, an optional in-body video companion, and a folded-in "Point releases"
// section for that cycle's patches. The footer carries both the Discussions
// prompt (from the old blog shell) and the pointer to the terse, exhaustive
// CHANGELOG.md (from the old changelog shell) - News is the story, CHANGELOG.md
// is the complete machine reference. basePath is inlined at config time.
function newsPostShell(title, basePath, opts = {}) {
    const t = escapeAttr(title);
    const b = escapeAttr(basePath);
    const desc = opts.description
        ? `\n        <meta name="description" content="${escapeAttr(
              opts.description
          )}" />`
        : "";
    const date = escapeAttr(opts.date || "");
    const version = escapeAttr(opts.version || "");
    // A sticky section TOC built at build time from the post's h2/h3 headings,
    // so it works with no JS and is SEO-visible; news.ts adds scroll-spy on top.
    // h3s indent under their h2. Rendered only when the post has sections.
    const headings = opts.headings || [];
    const toc = headings.length
        ? `<nav class="news__toc" aria-label="On this page">
                    <p class="news__toc-title">On this page</p>
                    ${headings
                        .map(
                            (h) =>
                                `<a href="#${escapeAttr(h.id)}" class="news__toc-link${
                                    h.level === "h3" ? " news__toc-link--sub" : ""
                                }">${escapeAttr(h.text)}</a>`
                        )
                        .join("\n                    ")}
                </nav>`
        : "";
    const layoutOpen = headings.length
        ? `<div class="news">
                ${toc}
                <article class="prose news__body">`
        : `<article class="prose">`;
    const layoutClose = headings.length ? `</article>
            </div>` : `</article>`;
    return `<!doctype html>
<html lang="en">
    <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>${t} - Nova Protocol</title>${desc}
        <link rel="icon" href="${b}favicon.svg" />
    </head>
    <body>
        <div id="header"></div>
        <main>
            ${layoutOpen}
                <p class="prose__meta">
                    <a href="${b}news/">&larr; News</a>
                    &nbsp;//&nbsp; ${date} &nbsp;//&nbsp; ${version}
                </p>
                <h1>${t}</h1>
                <div id="doc-body"></div>
                <footer class="post-footer">
                    <p class="post-footer__discuss">
                        Got a reaction, a question, or a ship you want to show
                        off?
                        <a
                            href="https://github.com/alexjercan/nova-protocol/discussions"
                            target="_blank"
                            rel="noopener"
                            >Talk about this release on GitHub Discussions</a
                        >.
                    </p>
                    <p class="post-footer__discuss">
                        Want the terse, complete list for every version
                        (patch releases included)?
                        <a
                            href="https://github.com/alexjercan/nova-protocol/blob/master/CHANGELOG.md"
                            target="_blank"
                            rel="noopener"
                            >Read CHANGELOG.md on GitHub</a
                        >.
                    </p>
                    <p class="post-footer__nav">
                        <a href="${b}news/">&larr; All news</a>
                    </p>
                </footer>
            ${layoutClose}
        </main>
        <div id="footer"></div>
    </body>
</html>`;
}

// Build one HtmlWebpackPlugin for a markdown news post, served at
// `/news/<slug>/` on the `news` chunk. `date`/`version` fill the meta line;
// `description` the head meta. The page title comes from the markdown H1.
function newsPostPage({
    slug,
    mdPath,
    title,
    date,
    version,
    description,
    publicPath,
}) {
    const abs = path.resolve(__dirname, mdPath);
    const { html, title: h1, headings } = renderMarkdownFile(abs);
    const pageTitle = title || h1;
    return new HtmlWebpackPlugin({
        filename: `news/${slug}/index.html`,
        chunks: ["news"],
        basePath: publicPath,
        docBody: html,
        templateContent: newsPostShell(pageTitle, publicPath, {
            date,
            version,
            description,
            headings,
        }),
    });
}

module.exports = {
    renderMarkdownFile,
    wikiDocPage,
    newsPostPage,
};
