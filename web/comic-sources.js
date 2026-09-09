const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { validatePageDefinition } = require("./comic-definitions");
const {
    validateScript,
    compileScript,
    scriptMarkdown,
} = require("./comic-script-build");

const SOURCE_ROOT = path.resolve(__dirname, "src/comics");
const ID = /^[a-z0-9][a-z0-9-]*$/;

function fields(value, names, owner) {
    if (!value || typeof value !== "object" || Array.isArray(value))
        throw new Error(`${owner} must be an object`);
    for (const name of names)
        if (!(name in value)) throw new Error(`${owner} has no ${name}`);
    for (const name of Object.keys(value))
        if (!names.includes(name))
            throw new Error(`${owner} has unknown field ${name}`);
}
function text(value, owner) {
    if (typeof value !== "string" || !value.trim())
        throw new Error(`${owner} needs text`);
}
function record(file) {
    if (!fs.existsSync(file)) throw new Error(`Missing comic source: ${file}`);
    return JSON.parse(fs.readFileSync(file, "utf8"));
}
function directories(root) {
    return fs
        .readdirSync(root, { withFileTypes: true })
        .filter((e) => e.isDirectory())
        .map((e) => e.name);
}

/** Read required season/episode metadata without executing any episode generator. */
function discoverComics(root = SOURCE_ROOT) {
    const seasons = directories(root)
        .map((name) => {
            if (!ID.test(name)) throw new Error(`Invalid season id: ${name}`);
            const directory = path.join(root, name);
            const comic = record(path.join(directory, "comic.json"));
            fields(comic, ["sequence", "title", "summary", "episodes"], name);
            for (const field of ["title", "summary"])
                text(comic[field], `${name}/${field}`);
            if (!Number.isSafeInteger(comic.sequence) || comic.sequence < 1)
                throw new Error("Season needs a positive sequence");
            if (!Array.isArray(comic.episodes) || !comic.episodes.length)
                throw new Error("Season needs episodes");
            const ids = new Set();
            const episodes = comic.episodes.map((id, index) => {
                if (typeof id !== "string" || !ID.test(id) || ids.has(id))
                    throw new Error("Invalid or duplicate episode id");
                ids.add(id);
                const episodeDirectory = path.join(directory, id);
                const episode = record(
                    path.join(episodeDirectory, "episode.json")
                );
                fields(
                    episode,
                    [
                        "title",
                        "summary",
                        "publication",
                        "pageCount",
                        "source",
                        "art",
                        "cover",
                        "coverAlt",
                    ],
                    `${name}/${id}`
                );
                for (const field of ["title", "summary", "coverAlt"])
                    text(episode[field], `${name}/${id}/${field}`);
                if (!["draft", "released"].includes(episode.publication))
                    throw new Error("Unknown episode publication state");
                if (
                    !Number.isSafeInteger(episode.pageCount) ||
                    episode.pageCount < 1
                )
                    throw new Error("Episode needs a positive pageCount");
                if (
                    typeof episode.source !== "string" ||
                    !/^[a-z][a-z0-9_-]*\.ts$/.test(episode.source)
                )
                    throw new Error("Invalid episode source");
                if (
                    !Array.isArray(episode.art) ||
                    !episode.art.length ||
                    new Set(episode.art).size !== episode.art.length ||
                    episode.art.some(
                        (file) =>
                            typeof file !== "string" ||
                            !/^art\/[a-z][a-z0-9_-]*\.py$/.test(file)
                    )
                )
                    throw new Error("Invalid episode art modules");
                if (
                    typeof episode.cover !== "string" ||
                    !ID.test(episode.cover)
                )
                    throw new Error("Invalid episode cover scene");
                const source = path.join(episodeDirectory, episode.source);
                const art = episode.art.map((file) =>
                    path.join(episodeDirectory, file)
                );
                for (const file of [source, ...art])
                    if (!fs.existsSync(file))
                        throw new Error(`Missing episode source: ${file}`);
                return { ...episode, id, number: index + 1, source, art };
            });
            if (directories(directory).some((id) => !ids.has(id)))
                throw new Error(`Unregistered episode directory in ${name}`);
            return { ...comic, path: name, episodes };
        })
        .sort((a, b) => a.sequence - b.sequence);
    if (new Set(seasons.map((s) => s.sequence)).size !== seasons.length)
        throw new Error("Duplicate season sequence");
    let draftSeen = false;
    for (const season of seasons)
        for (const episode of season.episodes) {
            if (episode.publication === "draft") draftSeen = true;
            else if (draftSeen)
                throw new Error(
                    "Released episodes must precede drafts in reading order"
                );
        }
    return seasons;
}

function execute(command, args) {
    const result = spawnSync(command, args, {
        encoding: "utf8",
        maxBuffer: 8_000_000,
    });
    if (result.status !== 0)
        throw new Error(
            `Comic generation failed: ${result.stderr || result.error}`
        );
    return result.stdout;
}
function put(file, content) {
    if (!fs.existsSync(file) || fs.readFileSync(file, "utf8") !== content)
        fs.writeFileSync(file, content);
}
/** Evaluate only the selected TS entry. Python receives scene names, never story text. */
function generateEpisode(episode, output) {
    const script = JSON.parse(
        execute(process.execPath, [
            path.join(__dirname, "read-comic-script.js"),
            episode.source,
        ])
    );
    validateScript(script, episode.pageCount);
    const illustrated = script.pages.filter((p) => p.kind === "illustrated");
    if (!illustrated.length) throw new Error("Invalid illustrated page count");
    if (
        episode.publication === "released" &&
        illustrated.length !== episode.pageCount
    )
        throw new Error("Released episode is not fully illustrated");
    const requested = [
        ...new Set([
            episode.cover,
            ...illustrated.flatMap((p) => p.panels.map((p) => p.art)),
        ]),
    ];
    execute("python3", [
        path.join(__dirname, "build-comics.py"),
        ...episode.art.flatMap((file) => ["--source", file]),
        "--scenes",
        JSON.stringify(requested),
        "--output",
        output,
    ]);
    const pages = compileScript(
        script,
        record(path.join(output, "scenes.json")),
        record(path.join(output, "palette.json"))
    );
    put(path.join(output, "pages.json"), JSON.stringify(pages, null, 2) + "\n");
    put(path.join(output, "SCRIPT.md"), scriptMarkdown(script, episode.title));
}

/** Select publication BEFORE generation; emit only referenced assets from selected episodes. */
function compileComics({
    root = SOURCE_ROOT,
    output,
    includeDrafts = false,
    generate = generateEpisode,
}) {
    root = path.resolve(root);
    output = path.resolve(output);
    const repository = path.resolve(__dirname, "..");
    const cache = path.join(__dirname, ".cache/story");
    if (
        output === root ||
        root.startsWith(output + path.sep) ||
        output.startsWith(root + path.sep) ||
        ((output === repository || output.startsWith(repository + path.sep)) &&
            !output.startsWith(cache + path.sep))
    )
        throw new Error("Comic output must be an isolated generated cache");
    const selected = discoverComics(root)
        .map((comic) => ({
            ...comic,
            episodes: comic.episodes.filter(
                (e) => includeDrafts || e.publication === "released"
            ),
        }))
        .filter((c) => c.episodes.length);
    const assets = new Map(),
        retained = new Set();
    fs.mkdirSync(output, { recursive: true });
    const comics = selected.map((comic) => {
        const episodes = comic.episodes.map((episode) => {
            const directory = path.join(output, comic.path, episode.id);
            generate(episode, directory);
            const assertAsset = (name) => {
                if (
                    typeof name !== "string" ||
                    !/^[a-z0-9][a-z0-9-]*\.(svg|png|webp)$/.test(name)
                )
                    throw new Error("Invalid generated comic asset");
                const file = path.join(directory, name);
                if (!fs.existsSync(file))
                    throw new Error(`Missing generated asset: ${file}`);
                assets.set(
                    `story/assets/${comic.path}/${episode.id}/${name}`,
                    file
                );
                retained.add(file);
            };
            const cover = `${episode.cover}.svg`;
            assertAsset(cover);
            const pageFile = path.join(directory, "pages.json");
            for (const name of [
                "pages.json",
                "scenes.json",
                "palette.json",
                "SCRIPT.md",
            ])
                retained.add(path.join(directory, name));
            const pages = record(pageFile);
            if (
                !Array.isArray(pages) ||
                !pages.length ||
                pages.length > episode.pageCount
            )
                throw new Error("Invalid illustrated page count");
            if (
                episode.publication === "released" &&
                pages.length !== episode.pageCount
            )
                throw new Error("Released episode is not fully illustrated");
            const ids = new Set();
            for (const page of pages) {
                fields(
                    page,
                    ["id", "title", "transcript", "definition"],
                    "generated page"
                );
                if (
                    typeof page.id !== "string" ||
                    !ID.test(page.id) ||
                    ids.has(page.id)
                )
                    throw new Error("Invalid or duplicate page id");
                ids.add(page.id);
                text(page.title, "Page title");
                text(page.transcript, "Page transcript");
                validatePageDefinition(page.definition, assertAsset);
                if (page.definition.layout === "panels") {
                    for (const panel of page.definition.panels)
                        panel.image = `${episode.id}/${panel.image}`;
                } else
                    page.definition.image = `${episode.id}/${page.definition.image}`;
            }
            const { source, art, ...metadata } = episode;
            return { ...metadata, cover: `${episode.id}/${cover}`, pages };
        });
        return {
            ...comic,
            episodes,
            cover: episodes[0].cover,
            coverAlt: episodes[0].coverAlt,
        };
    });
    function prune(directory) {
        for (const entry of fs.readdirSync(directory, {
            withFileTypes: true,
        })) {
            const file = path.join(directory, entry.name);
            if (entry.isDirectory()) {
                prune(file);
                if (!fs.readdirSync(file).length) fs.rmdirSync(file);
            } else if (!retained.has(file)) fs.unlinkSync(file);
        }
    }
    prune(output);
    return { comics, assets };
}

module.exports = { SOURCE_ROOT, discoverComics, compileComics };
