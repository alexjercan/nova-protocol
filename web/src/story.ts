import "./style.css";
import { initSite } from "./site";
import { loadComicPages, readComicManifest } from "./comics/comic-catalog";
import { renderComicPage } from "./comics/comic-renderer";
import { ComicPlayer } from "./story-reader";

initSite();

const root = document.querySelector<HTMLElement>("[data-comic-reader]");
const manifest = readComicManifest();
if (root && manifest) {
    const definitions = loadComicPages(manifest);
    const pages = definitions.map((page, index) => {
        const entry = manifest.pages[index];
        return renderComicPage(page, {
            id: entry.id,
            number: index + 1,
            comicPath: manifest.path,
            basePath: manifest.basePath,
            chapter: { title: entry.chapter, state: entry.state },
        });
    });
    new ComicPlayer({
        root,
        pages,
        initialPage: window.location.hash.slice(1),
    });
}
