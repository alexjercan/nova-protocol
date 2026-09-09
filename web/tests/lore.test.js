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

for (const [slug, title] of [
    ["places/keystone", "Keystone"],
    ["places/aquila", "Aquila"],
    ["places/baikal", "Baikal"],
    ["ships/kaveri", "Kaveri"],
    ["ships/ebro", "Ebro"],
    ["ships/gantry", "Gantry"],
    ["ships/bastion", "Bastion"],
    ["ships/foundation", "Foundation"],
    ["ships/altair", "Altair"],
    ["ships/redress", "Redress"],
    ["ships/windfall", "Windfall"],
    ["characters/nadia-sen", "Nadia Sen"],
    ["characters/owen-park", "Owen Park"],
    ["characters/ivo-marin", "Ivo Marin"],
    ["characters/elena-ward", "Elena Ward"],
]) {
    const page = lore.pages.find((entry) => entry.slug === slug);
    assert.equal(page?.title, title, `${slug} uses its approved name`);
    assert.equal(
        pages.get(slug + "/").title,
        title,
        `${slug} has its approved article title`
    );
}
for (const crew of ["nadia-sen", "owen-park", "ivo-marin"]) {
    assert(
        pages
            .get("ships/gantry/")
            .html.includes(`href="../../characters/${crew}/"`),
        `Gantry links to ${crew}'s identity`
    );
    assert(
        pages
            .get(`characters/${crew}/`)
            .html.includes('href="../../ships/gantry/"'),
        `${crew}'s identity links back to Gantry`
    );
}
for (const crew of ["nadia-sen", "owen-park", "ivo-marin"]) {
    assert(
        pages
            .get(`characters/${crew}/`)
            .html.includes(
                `src="../../../assets/lore/${crew}-portrait-concept.svg"`
            ),
        `${crew} has a starting-identity portrait`
    );
}
assert(
    pages
        .get("ships/gantry/")
        .html.includes('src="../../../assets/lore/gantry-design-concept.svg"'),
    "Gantry links its intact shared-model design sheet"
);
for (const subject of [
    "organizations/clearwell-waterworks/",
    "places/baikal/",
    "characters/jonah-mercer/",
]) {
    const founder = "characters/elena-ward/";
    for (const [from, to] of [
        [founder, subject],
        [subject, founder],
    ]) {
        const href = path.posix.relative(from, to) + "/";
        assert(
            pages.get(from).html.includes(`href="${href}"`),
            `${from} links to ${to}'s identity`
        );
    }
}
for (const [ship, partner] of [
    ["redress", "windfall"],
    ["windfall", "redress"],
]) {
    assert(
        pages.get(`ships/${ship}/`).html.includes(`href="../${partner}/"`),
        `${ship} links to its working partner`
    );
}
for (const slug of ["places/junction", "places/clearwell"]) {
    assert(!slugs.has(slug), `${slug} is not a current article`);
    assert(
        !fs.existsSync(path.join(source, slug + ".md")),
        `${slug} has no duplicate source`
    );
}
assert(
    !lore.pages.some(
        (page) => page.md === "README.md" || page.md.startsWith("seasons/")
    ),
    "authoring guidance and unreleased season outcomes stay unpublished"
);

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
    if (/^(places|organizations|ships|characters)\//.test(page.slug)) {
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
