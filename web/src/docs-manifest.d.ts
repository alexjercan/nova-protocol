// Types for docs-manifest.js, the single source of truth for both doc
// sections (/wiki/ and /create/). The data lives in plain CommonJS so the
// webpack config can require it at configure time; this declaration gives the
// browser chunk (docs.ts) full typing over the same objects.

export interface DocPage {
    // URL segment under the section root, e.g. "actions" -> /create/actions/.
    slug: string;
    // Markdown source, relative to the section's mdDir.
    md: string;
    title: string;
    // Sidebar group; must be one of the section's categories.
    category: string;
    // Small controlled taxonomy - tag chips, search, and the auto
    // "shares a tag" half of See also.
    tags: string[];
    // One line - index cards, search results, and the meta description.
    summary: string;
    // Explicit cross-links (same-section slugs), shown first under See also.
    related: string[];
    // Section headings, so search matches on in-page topics too.
    headings: string[];
    // Slug of the parent page: sidebar nesting, crumbs, and the parent's
    // #wiki-children grid.
    parent?: string;
    // Icon asset for the parent's child grid (placeholder until captured).
    icon?: string;
    // Render the build-time contents box (reference pages opt in).
    toc?: boolean;
    // Not yet written - rendered as a muted, non-navigable "coming soon" entry.
    comingSoon?: boolean;
}

export interface DocLanding {
    md: string;
    title: string;
    description?: string;
}

export interface DocSection {
    // URL root: "wiki" -> /wiki/.
    root: string;
    title: string;
    // Sidebar home-link label ("Wiki index" / "Create home").
    homeLabel: string;
    searchPlaceholder: string;
    // <title> suffix for the section's pages.
    titleSuffix: string;
    // Markdown directory, relative to web/ ("src/wiki").
    mdDir: string;
    // A markdown page rendered at the section root itself; absent when the
    // section keeps a hand-authored index shell instead.
    landing?: DocLanding;
    // Ordered sidebar groups; every page's category must be listed here.
    categories: string[];
    pages: DocPage[];
}

export const DOC_SECTIONS: DocSection[];
