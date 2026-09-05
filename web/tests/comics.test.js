const assert = require("node:assert/strict");
const path = require("node:path");
const { discoverComics, pagesOf, validateComic } = require("../comic-build");

const comics = discoverComics();
const nova = comics.find((comic) => comic.path === "nova-protocol");
assert.ok(nova, "path-indexed Nova Protocol comic is discovered");
assert.equal(nova.chapters.length, 2, "manifest keeps two chapters");
assert.deepEqual(
    pagesOf(nova).map((page) => page.id),
    ["cover", "released", "no-fleet-code", "the-same-belt", "the-record"],
    "manifest order is the reading order"
);
assert.equal("slug" in nova, false, "directory path is the only comic index");

// The validator is the whole guarantee `docs/development.md` advertises, so
// what it REFUSES matters more than what it accepts.
const root = path.resolve(__dirname, "../src/comics");
const good = () => ({
    title: "T",
    summary: "S",
    status: "Draft",
    cover: "first-shift.svg",
    coverAlt: "A",
    chapters: [
        {
            id: "one",
            title: "One",
            pages: [{ id: "cover", title: "Cover", source: "pages/cover" }],
        },
    ],
});

function refuses(name, mutate) {
    const comic = good();
    mutate(comic);
    assert.throws(
        () => validateComic("nova-protocol", comic, root),
        undefined,
        name
    );
}

validateComic("nova-protocol", good(), root);
refuses("a chapter with no id at all", (c) => delete c.chapters[0].id);
refuses("a page with no id at all", (c) => delete c.chapters[0].pages[0].id);
refuses("a missing coverAlt", (c) => delete c.coverAlt);
refuses("a page source outside pages/", (c) => {
    c.chapters[0].pages[0].source = "cover";
});
refuses("a page source nested under pages/", (c) => {
    c.chapters[0].pages[0].source = "pages/deep/cover";
});
refuses("a page module that does not exist", (c) => {
    c.chapters[0].pages[0].source = "pages/nothing-here";
});
refuses("a duplicate page id", (c) => {
    c.chapters[0].pages.push({
        id: "cover",
        title: "Again",
        source: "pages/cover",
    });
});
assert.throws(
    () => validateComic("Not An Id", good(), root),
    undefined,
    "a directory name that is not a valid public id"
);

console.log("comics.test.js: all assertions passed");
