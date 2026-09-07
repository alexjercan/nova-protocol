const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { renderMarkdownFile } = require("../markdown");
const { DOC_SECTIONS } = require("../src/docs-manifest");

const lore = DOC_SECTIONS.find((section) => section.root === "lore");
assert(lore?.landing, "lore has one landing page");
const source = path.resolve(__dirname, "..", lore.mdDir);
const landing = renderMarkdownFile(path.join(source, lore.landing.md));
const pages = new Map([
    ["", landing],
    ...lore.pages.map((page) => [
        page.slug + "/",
        renderMarkdownFile(path.join(source, page.md)),
    ]),
]);
const slugs = new Set(lore.pages.map((page) => page.slug));
assert.equal(slugs.size, lore.pages.length, "article URLs are unique");

for (const category of lore.categories) {
    const heading = landing.headings.find((entry) => entry.text === category);
    assert(heading, `the directory includes ${category}`);
    const start = landing.html.indexOf(`<h2 id="${heading.id}"`);
    const end = landing.html.indexOf("<h2 ", start + 1);
    const group = landing.html.slice(start, end < 0 ? undefined : end);
    const members = lore.pages.filter((page) => page.category === category);
    assert(members.length, `${category} is not an empty directory group`);
    for (const page of members) {
        assert(
            group.includes(`href="${page.slug}/"`),
            `${page.title} appears in its directory group`
        );
    }
}

const introduction = landing.html.slice(0, landing.html.indexOf("<h2 "));
for (const page of lore.pages) {
    assert(
        lore.categories.includes(page.category),
        `${page.title} has a category`
    );
    for (const related of page.related) {
        assert(
            slugs.has(related),
            `${page.title} relates to a published article`
        );
    }
    const rendered = pages.get(page.slug + "/");
    for (const heading of page.headings) {
        assert(
            rendered.headings.some((entry) => entry.text === heading),
            `${page.title} has its searchable heading ${heading}`
        );
    }
    if (/^(places|organizations|characters)\//.test(page.slug)) {
        assert(
            !introduction.includes(page.title),
            `${page.title} belongs in the directory, not the abstract introduction`
        );
    }
}

let checked = 0;
for (const prefix of ["/", "/nova-protocol/"]) {
    const root = `${prefix}lore/`;
    const sitePages = new Set([
        `${prefix}story/`,
        ...DOC_SECTIONS.flatMap((section) => [
            `${prefix}${section.root}/`,
            ...section.pages.map(
                (page) => `${prefix}${section.root}/${page.slug}/`
            ),
        ]),
    ]);
    for (const [slug, page] of pages) {
        const url = new URL(root + slug, "https://lore.test");
        for (const [, href] of page.html.matchAll(
            /\b(?:href|src)="([^"]+)"/g
        )) {
            const target = new URL(href, url);
            if (target.origin !== url.origin) continue;
            assert(
                target.pathname.startsWith(prefix),
                `${href} stays under ${prefix}`
            );
            if (target.pathname.startsWith(root)) {
                const article = pages.get(target.pathname.slice(root.length));
                assert(
                    article,
                    `${slug}: ${href} resolves to a published article`
                );
                if (target.hash) {
                    const id = decodeURIComponent(target.hash.slice(1));
                    assert(
                        article.html.includes(`id="${id}"`),
                        `${href} has its fragment`
                    );
                }
            } else if (target.pathname.startsWith(`${prefix}assets/`)) {
                const asset = target.pathname.slice(prefix.length);
                assert(
                    fs
                        .statSync(path.resolve(__dirname, "../src", asset))
                        .isFile(),
                    `${href} resolves to an asset`
                );
            } else {
                assert(
                    sitePages.has(target.pathname),
                    `${href} resolves to a site page`
                );
            }
            checked++;
        }
    }
}

console.log(`lore.test.js: ${pages.size} pages and ${checked} links passed`);
