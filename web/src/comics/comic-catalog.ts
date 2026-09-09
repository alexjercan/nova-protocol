import type { ComicPage } from "./comic-page";

/** One generated page, embedded only after the build selects its episode. */
export interface ComicManifestPage {
    id: string;
    title: string;
    transcript: string;
    definition: ComicPage;
}

/** Build-selected reader data; authoring scripts never enter the browser bundle. */
export interface ComicManifest {
    path: string;
    title: string;
    episodeTitle: string;
    basePath: string;
    pages: ComicManifestPage[];
}

/** Read the embedded episode, or return null on an archive page without a reader. */
export function readComicManifest(
    documentRoot: Document = document
): ComicManifest | null {
    const script = documentRoot.getElementById("comic-definition");
    if (!script?.textContent) return null;
    return JSON.parse(script.textContent) as ComicManifest;
}

/** Load the selected definitions without a recursive import of comic source folders. */
export function loadComicPages(manifest: ComicManifest): ComicPage[] {
    return manifest.pages.map((page) => page.definition);
}
