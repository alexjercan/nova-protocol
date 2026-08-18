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

function escapeHtml(s) {
    return String(s)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

// Semantic highlighter for ```ron fences. highlight.js has no RON grammar, so
// every RON snippet used to render as plain escaped text; this small tokenizer
// understands just enough of the real lexical shape to color it like an IDE
// would. Classes (styled in src/style.css):
// - ron-type:    PascalCase tagged type/variant names (Campaign, Some, None)
// - ron-key:     field keys - an ident directly followed by ":"
// - ron-string:  double-quoted strings, escapes included
// - ron-literal: numbers, true, false
// - ron-comment: // line and /* block */ comments
// - ron-punct:   structural punctuation, dimmed
// Everything is HTML-escaped; concatenating the span texts always reproduces
// the escaped source exactly (the tokenizer never drops or reorders input).
function highlightRon(code) {
    const span = (cls, text) =>
        `<span class="${cls}">${escapeHtml(text)}</span>`;
    const isIdentStart = (c) => /[A-Za-z_]/.test(c);
    const isIdent = (c) => /[A-Za-z0-9_]/.test(c);
    const n = code.length;
    let out = "";
    let i = 0;
    while (i < n) {
        const c = code[i];
        if (c === "/" && code[i + 1] === "/") {
            let j = code.indexOf("\n", i);
            if (j === -1) j = n;
            out += span("ron-comment", code.slice(i, j));
            i = j;
        } else if (c === "/" && code[i + 1] === "*") {
            let j = code.indexOf("*/", i + 2);
            j = j === -1 ? n : j + 2;
            out += span("ron-comment", code.slice(i, j));
            i = j;
        } else if (c === '"') {
            let j = i + 1;
            while (j < n && code[j] !== '"') {
                if (code[j] === "\\") j++;
                j++;
            }
            j = Math.min(j + 1, n);
            out += span("ron-string", code.slice(i, j));
            i = j;
        } else if (
            /\d/.test(c) ||
            (c === "-" && /[\d.]/.test(code[i + 1] || "")) ||
            (c === "." && /\d/.test(code[i + 1] || ""))
        ) {
            const m =
                /^-?(?:0x[0-9a-fA-F_]+|0b[01_]+|\d[\d_]*(?:\.\d*)?(?:[eE][+-]?\d+)?|\.\d+(?:[eE][+-]?\d+)?)/.exec(
                    code.slice(i)
                );
            out += span("ron-literal", m[0]);
            i += m[0].length;
        } else if (isIdentStart(c)) {
            let j = i + 1;
            while (j < n && isIdent(code[j])) j++;
            const word = code.slice(i, j);
            let k = j;
            while (k < n && (code[k] === " " || code[k] === "\t")) k++;
            let cls = null;
            if (code[k] === ":") cls = "ron-key";
            else if (word === "true" || word === "false") cls = "ron-literal";
            else if (/^[A-Z]/.test(word)) cls = "ron-type";
            out += cls ? span(cls, word) : escapeHtml(word);
            i = j;
        } else if (/[(){}[\]:,]/.test(c)) {
            let j = i + 1;
            while (j < n && /[(){}[\]:,]/.test(code[j])) j++;
            out += span("ron-punct", code.slice(i, j));
            i = j;
        } else {
            out += escapeHtml(c);
            i += 1;
        }
    }
    return out;
}

// Build-time markdown -> HTML for the doc pages (/wiki/ and /create/).
// Rendering happens here in Node (the webpack config calls docPage at
// configure time), so there is no runtime markdown cost and a no-JS / SEO
// reader still gets the full article - the same "content in HTML, chrome via
// JS" split the hand-authored pages use.
//
// - A fenced ```mermaid block becomes a <pre class="mermaid"> holding the escaped
//   diagram source; docs.ts renders it client-side (mermaid needs the DOM). A
//   ```ron fence goes through highlightRon above (highlight.js has no RON). Every
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
        if (lang === "ron") {
            return `<pre><code class="hljs language-ron">${highlightRon(code)}</code></pre>`;
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

// A MediaWiki-style "Contents" box for reference pages, built at build time
// from the same h2/h3 list markdown-it-anchor produced - so the links can never
// drift from the real in-page anchors. h3s nest under their h2 as a sub-list.
function tocBox(headings) {
    if (!headings || headings.length === 0) return "";
    const items = [];
    let open = false;
    for (const h of headings) {
        const link = `<a href="#${escapeAttr(h.id)}">${escapeAttr(h.text)}</a>`;
        if (h.level === "h2") {
            if (open) items.push("</ol></li>");
            items.push(`<li>${link}`);
            open = true;
            items.push("<ol>");
        } else if (open) {
            items.push(`<li>${link}</li>`);
        } else {
            // An h3 before any h2 still gets a top-level row.
            items.push(`<li>${link}`);
            open = true;
            items.push("<ol>");
        }
    }
    if (open) items.push("</ol></li>");
    return `<nav class="wiki-toc" aria-label="Contents">
                        <p class="wiki-toc__title">Contents</p>
                        <ol>${items.join("")}</ol>
                    </nav>`;
}

// The page shell for a markdown doc: the same chrome as a hand-authored doc
// page (header/footer placeholders, the manifest-driven #wiki-nav aside, the
// crumb/h1/#wiki-tags, and #wiki-seealso), with a #doc-body placeholder the
// partials plugin fills with the rendered markdown after templating - so lodash
// never runs over code samples. Unlike a `template` FILE, a templateContent
// STRING is not run through lodash, so basePath is inlined here at config time
// (publicPath is already known) rather than left as a <%= %> token.
// opts: { section, description, crumbParent: { slug, title }, toc: headings,
// landing }. `section` ({ root, title, titleSuffix }) picks the crumb root and
// <title> suffix, so /wiki/ and /create/ pages share this one shell. A
// description is rendered as the page meta; a crumbParent renders a two-level
// crumb ("Create / <parent> / <title>") for child pages; toc renders the
// contents box above the body (the reference pages opt in); landing renders the
// section root page itself, so the crumb (which would just self-link) is
// omitted.
function docShell(title, basePath, opts = {}) {
    const t = escapeAttr(title);
    const b = escapeAttr(basePath);
    const section = opts.section;
    const root = escapeAttr(section.root);
    const sectionTitle = escapeAttr(section.title);
    const desc = opts.description
        ? `\n        <meta name="description" content="${escapeAttr(
              opts.description
          )}" />`
        : "";
    const parent = opts.crumbParent;
    const crumb = parent
        ? `<a href="${b}${root}/">${sectionTitle}</a>
                        / <a href="${b}${root}/${escapeAttr(parent.slug)}/">${escapeAttr(
                            parent.title
                        )}</a>
                        / ${t}`
        : `<a href="${b}${root}/">${sectionTitle}</a>
                        / ${t}`;
    const crumbBlock = opts.landing
        ? ""
        : `<p class="wiki__crumb">
                        ${crumb}
                    </p>
                    `;
    const htmlTitle = opts.landing
        ? `${t} - Nova Protocol`
        : `${t} - ${escapeAttr(section.titleSuffix)}`;
    return `<!doctype html>
<html lang="en">
    <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>${htmlTitle}</title>${desc}
        <link rel="icon" href="${b}favicon.svg" />
    </head>
    <body>
        <div id="header"></div>
        <main>
            <div class="wiki">
                <aside
                    class="wiki__nav"
                    id="wiki-nav"
                    aria-label="${sectionTitle} navigation"
                ></aside>
                <article class="wiki__body prose">
                    ${crumbBlock}<h1>${t}</h1>
                    <div class="wiki__tags" id="wiki-tags"></div>
                    ${opts.toc ? tocBox(opts.toc) : ""}
                    <div id="doc-body"></div>
                    <div id="wiki-seealso"></div>
                </article>
            </div>
        </main>
        <div id="footer"></div>
    </body>
</html>`;
}

// Build one HtmlWebpackPlugin for a markdown doc page in a doc SECTION (/wiki/
// or /create/). The rendered body rides on the plugin's `docBody` option;
// HtmlPartialsPlugin injects it into the #doc-body placeholder at beforeEmit
// (see webpack-partials.js). Shares the `docs` chunk so the sidebar/search/
// tags/see-also all render from the manifest. `description` sets the page
// meta; `crumbParent` renders a child crumb; `toc: true` renders the auto
// contents box; `landing: true` emits the page at the section root itself.
function docPage({
    section,
    slug,
    mdPath,
    title,
    description,
    crumbParent,
    toc,
    landing,
    publicPath,
}) {
    const abs = path.resolve(__dirname, mdPath);
    const { html, title: h1, headings } = renderMarkdownFile(abs);
    const pageTitle = title || h1;
    const filename = landing
        ? `${section.root}/index.html`
        : `${section.root}/${slug}/index.html`;
    return new HtmlWebpackPlugin({
        filename,
        chunks: ["docs"],
        basePath: publicPath,
        docBody: html,
        templateContent: docShell(pageTitle, publicPath, {
            section,
            description,
            crumbParent,
            toc: toc ? headings : undefined,
            landing,
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
                                    h.level === "h3"
                                        ? " news__toc-link--sub"
                                        : ""
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
    const layoutClose = headings.length
        ? `</article>
            </div>`
        : `</article>`;
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
    docPage,
    newsPostPage,
    highlightRon,
};
