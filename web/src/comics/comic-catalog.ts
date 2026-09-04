import { ComicPage } from "./comic-page";

export interface ComicManifestPage {
    id: string;
    title: string;
    source: string;
    chapter: string;
}

export interface ComicManifest {
    path: string;
    title: string;
    summary: string;
    status: string;
    cover: string;
    basePath: string;
    pages: ComicManifestPage[];
}

interface PageModule {
    default: ComicPage;
}

interface RequireContext {
    (id: string): PageModule;
    keys(): string[];
}

declare const require: NodeRequire & {
    context(
        directory: string,
        useSubdirectories: boolean,
        regExp: RegExp
    ): RequireContext;
};

const pageModules = require.context(".", true, /\/pages\/[^/]+\.ts$/);

export function readComicManifest(
    documentRoot: Document = document
): ComicManifest | null {
    const script = documentRoot.getElementById("comic-definition");
    if (!script?.textContent) return null;
    return JSON.parse(script.textContent) as ComicManifest;
}

export function loadComicPages(manifest: ComicManifest): ComicPage[] {
    return manifest.pages.map((page) => {
        const key = `./${manifest.path}/${page.source}.ts`;
        if (!pageModules.keys().includes(key)) {
            throw new Error(`Comic page module is missing: ${key}`);
        }
        return pageModules(key).default;
    });
}
