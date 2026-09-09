export function relativePageIndex(
    current: number,
    change: number,
    pageCount: number
): number {
    if (pageCount < 1) return 0;
    return Math.max(0, Math.min(pageCount - 1, current + change));
}

export interface ComicPlayerOptions {
    root: HTMLElement;
    pages: HTMLElement[];
    transcripts: string[];
    initialPage?: string;
}

/** Read one episode with page-scoped transcripts and native image zoom. */
export class ComicPlayer {
    private readonly root: HTMLElement;
    private readonly viewport: HTMLElement;
    private readonly pages: HTMLElement[];
    private readonly transcripts: string[];
    private readonly pageLinks: HTMLAnchorElement[];
    private current = 0;
    private panelExports = "";

    constructor(options: ComicPlayerOptions) {
        const viewport = options.root.querySelector<HTMLElement>(
            "[data-page-viewport]"
        );
        if (
            !viewport ||
            !options.pages.length ||
            options.transcripts.length !== options.pages.length ||
            options.transcripts.some(
                (text) => typeof text !== "string" || !text.trim()
            )
        ) {
            throw new Error(
                "ComicPlayer needs a viewport, pages, and one transcript per page"
            );
        }
        this.root = options.root;
        this.viewport = viewport;
        this.pages = options.pages;
        this.transcripts = options.transcripts;
        this.pageLinks = Array.from(
            this.root.querySelectorAll<HTMLAnchorElement>("[data-page-link]")
        );
        this.viewport.replaceChildren(...this.pages);
        this.bindControls();
        this.setContents(
            this.root.dataset.contentsOpen === "true" &&
                !window.matchMedia("(max-width: 760px)").matches
        );
        this.open(options.initialPage);
    }

    next(): void {
        this.go(1);
    }
    previous(): void {
        this.go(-1);
    }

    open(pageId?: string): void {
        const requested = pageId
            ? this.pages.findIndex((page) => page.id === pageId)
            : 0;
        this.current = requested >= 0 ? requested : 0;
        this.updateState(false);
    }

    private setContents(open: boolean): void {
        this.root.dataset.contentsOpen = String(open);
        const contents =
            this.root.querySelector<HTMLElement>("#comic-contents");
        if (contents) contents.hidden = !open;
        this.root
            .querySelector("[data-contents-toggle]")
            ?.setAttribute("aria-expanded", String(open));
    }

    private bindControls(): void {
        for (const button of Array.from(
            this.root.querySelectorAll<HTMLButtonElement>(
                "[data-reader-dialog]"
            )
        )) {
            const dialog = this.root.querySelector<HTMLDialogElement>(
                `#${button.getAttribute("aria-controls")}`
            );
            if (!dialog) throw new Error("Missing reader dialog");
            button.addEventListener("click", () => dialog.showModal());
            dialog
                .querySelector("[data-dialog-close]")
                ?.addEventListener("click", () => dialog.close());
            dialog.addEventListener("close", () => button.focus());
        }
        this.root.addEventListener("comic:ready", () =>
            this.updateState(false)
        );
        this.root
            .querySelector("[data-art-toggle]")
            ?.addEventListener("click", (event) => {
                const enabled = this.root.dataset.artOnly !== "true";
                this.root.dataset.artOnly = String(enabled);
                (event.currentTarget as HTMLElement).setAttribute(
                    "aria-pressed",
                    String(enabled)
                );
            });
        window.addEventListener("hashchange", () =>
            this.open(window.location.hash.slice(1))
        );
        this.root
            .querySelector("[data-page-previous]")
            ?.addEventListener("click", () => this.previous());
        this.root
            .querySelector("[data-page-next]")
            ?.addEventListener("click", () => this.next());
        this.root
            .querySelector("[data-contents-toggle]")
            ?.addEventListener("click", () =>
                this.setContents(this.root.dataset.contentsOpen !== "true")
            );
        for (const link of this.pageLinks) {
            link.addEventListener("click", (event) => {
                event.preventDefault();
                const index = this.pages.findIndex(
                    (page) => page.id === link.dataset.pageLink
                );
                if (index >= 0) this.showPage(index);
                if (window.matchMedia("(max-width: 760px)").matches)
                    this.setContents(false);
            });
        }
        let wheelLocked = false;
        let wheelRelease: number | undefined;
        this.viewport.addEventListener(
            "wheel",
            (event) => {
                if (event.ctrlKey) return;
                event.preventDefault();
                window.clearTimeout(wheelRelease);
                wheelRelease = window.setTimeout(() => {
                    wheelLocked = false;
                }, 180);
                if (wheelLocked || Math.abs(event.deltaY) < 8) return;
                wheelLocked = true;
                this.go(event.deltaY > 0 ? 1 : -1);
            },
            { passive: false }
        );
        this.viewport.addEventListener("keydown", (event) => {
            if (
                event.ctrlKey ||
                event.metaKey ||
                event.altKey ||
                (event.target instanceof HTMLElement &&
                    event.target.closest(
                        "a,button,input,textarea,select,summary,[contenteditable]"
                    ))
            )
                return;
            const changes: Partial<Record<string, number>> = {
                ArrowDown: 1,
                ArrowRight: 1,
                PageDown: 1,
                " ": 1,
                ArrowUp: -1,
                ArrowLeft: -1,
                PageUp: -1,
            };
            const change = changes[event.key];
            if (change === undefined) return;
            event.preventDefault();
            this.go(change);
        });
        let touchStartY: number | null = null;
        this.viewport.addEventListener(
            "touchstart",
            (event) => {
                touchStartY =
                    event.touches.length === 1
                        ? event.touches[0].clientY
                        : null;
            },
            { passive: true }
        );
        this.viewport.addEventListener(
            "touchend",
            (event) => {
                if (touchStartY === null) return;
                const endY = event.changedTouches[0]?.clientY ?? touchStartY;
                const distance = touchStartY - endY;
                touchStartY = null;
                if (Math.abs(distance) >= 40) this.go(distance > 0 ? 1 : -1);
            },
            { passive: true }
        );
        this.viewport.addEventListener(
            "touchcancel",
            () => {
                touchStartY = null;
            },
            { passive: true }
        );
    }

    private go(change: number): void {
        this.showPage(
            relativePageIndex(this.current, change, this.pages.length)
        );
    }

    private showPage(index: number): void {
        this.current = relativePageIndex(index, 0, this.pages.length);
        this.updateState(true);
    }

    private updateState(updateHash: boolean): void {
        const page = this.pages[this.current];
        this.pages.forEach((candidate, index) => {
            candidate.hidden = index !== this.current;
        });
        const transcript = this.root.querySelector<HTMLElement>(
            "[data-page-transcript]"
        );
        if (
            transcript &&
            transcript.textContent !== this.transcripts[this.current]
        )
            transcript.textContent = this.transcripts[this.current];
        for (const label of Array.from(
            this.root.querySelectorAll("[data-dialog-page]")
        ))
            label.textContent = `Page ${String(this.current + 1).padStart(2, "0")} / ${this.pages.length}`;
        const art =
            this.root.querySelector<HTMLAnchorElement>("[data-page-art]");
        const picture = page.querySelector<HTMLImageElement>("img");
        if (art) {
            const source = page.dataset.artUrl || picture?.src;
            art.hidden = !source;
            if (source) art.href = source;
            else art.removeAttribute("href");
        }
        const panelButton = this.root.querySelector<HTMLButtonElement>(
            '[data-reader-dialog="panels"]'
        );
        const panelList = this.root.querySelector<HTMLElement>(
            "[data-panel-image-list]"
        );
        const panelExports = page.dataset.panelExports || "[]";
        if (panelButton && panelList && panelExports !== this.panelExports) {
            this.panelExports = panelExports;
            const panels = JSON.parse(panelExports) as {
                id: string;
                art: string;
                composed: string;
            }[];
            panelButton.hidden = !panels.length;
            panelList.replaceChildren(
                ...panels.map((panel) => {
                    const item = document.createElement("li");
                    item.append(`Panel ${panel.id.toUpperCase()}: `);
                    for (const [kind, label] of [
                        ["art", "Art SVG"],
                        ["composed", "Lettered SVG"],
                    ] as const) {
                        const link = document.createElement("a");
                        link.textContent = label;
                        link.href = panel[kind];
                        link.target = "_blank";
                        link.rel = "noopener";
                        link.dataset.panelImage = kind;
                        item.append(link, " ");
                    }
                    return item;
                })
            );
        }
        const currentLabel = this.root.querySelector<HTMLElement>(
            "[data-page-current]"
        );
        if (currentLabel)
            currentLabel.textContent = String(this.current + 1).padStart(
                2,
                "0"
            );
        const previous = this.root.querySelector<HTMLButtonElement>(
            "[data-page-previous]"
        );
        const next =
            this.root.querySelector<HTMLButtonElement>("[data-page-next]");
        if (previous) previous.disabled = this.current === 0;
        if (next) next.disabled = this.current === this.pages.length - 1;
        for (const link of this.pageLinks) {
            const active = link.dataset.pageLink === page.id;
            link.classList.toggle("is-active", active);
            if (active) link.setAttribute("aria-current", "page");
            else link.removeAttribute("aria-current");
        }
        page.dispatchEvent(new Event("comic:show"));
        if (updateHash && page.id)
            window.history.replaceState(null, "", `#${page.id}`);
    }
}
