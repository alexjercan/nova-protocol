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
    initialPage?: string;
}

/** Play an ordered list of rendered comic pages inside the shared HUD reader. */
export class ComicPlayer {
    private readonly root: HTMLElement;
    private readonly viewport: HTMLElement;
    private readonly pages: HTMLElement[];
    private readonly pageLinks: HTMLAnchorElement[];
    private current = 0;

    constructor(options: ComicPlayerOptions) {
        const viewport = options.root.querySelector<HTMLElement>(
            "[data-page-viewport]"
        );
        if (!viewport || !options.pages.length) {
            throw new Error(
                "ComicPlayer needs a viewport and at least one page"
            );
        }
        this.root = options.root;
        this.viewport = viewport;
        this.pages = options.pages;
        this.pageLinks = Array.from(
            this.root.querySelectorAll<HTMLAnchorElement>("[data-page-link]")
        );
        this.viewport.replaceChildren(...this.pages);
        this.bindControls();
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

    private bindControls(): void {
        this.root
            .querySelector<HTMLButtonElement>("[data-page-previous]")
            ?.addEventListener("click", () => this.previous());
        this.root
            .querySelector<HTMLButtonElement>("[data-page-next]")
            ?.addEventListener("click", () => this.next());

        const contentsToggle = this.root.querySelector<HTMLButtonElement>(
            "[data-contents-toggle]"
        );
        contentsToggle?.addEventListener("click", () => {
            const open = this.root.dataset.contentsOpen !== "true";
            this.root.dataset.contentsOpen = String(open);
            contentsToggle.setAttribute("aria-expanded", String(open));
        });

        for (const link of this.pageLinks) {
            link.addEventListener("click", (event) => {
                event.preventDefault();
                const index = this.pages.findIndex(
                    (page) => page.id === link.dataset.pageLink
                );
                if (index >= 0) this.showPage(index);
                if (window.matchMedia("(max-width: 760px)").matches) {
                    this.root.dataset.contentsOpen = "false";
                    contentsToggle?.setAttribute("aria-expanded", "false");
                }
            });
        }

        let wheelLocked = false;
        let wheelRelease: number | undefined;
        this.viewport.addEventListener(
            "wheel",
            (event) => {
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
                touchStartY = event.touches[0]?.clientY ?? null;
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

        if (window.matchMedia("(max-width: 760px)").matches) {
            this.root.dataset.contentsOpen = "false";
            contentsToggle?.setAttribute("aria-expanded", "false");
        }
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
        const currentLabel = this.root.querySelector<HTMLElement>(
            "[data-page-current]"
        );
        const progress = this.root.querySelector<HTMLElement>(
            "[data-page-progress]"
        );
        const previous = this.root.querySelector<HTMLButtonElement>(
            "[data-page-previous]"
        );
        const next =
            this.root.querySelector<HTMLButtonElement>("[data-page-next]");
        if (currentLabel) {
            currentLabel.textContent = String(this.current + 1).padStart(
                2,
                "0"
            );
        }
        if (progress) {
            progress.style.width = `${((this.current + 1) / this.pages.length) * 100}%`;
        }
        if (previous) previous.disabled = this.current === 0;
        if (next) next.disabled = this.current === this.pages.length - 1;
        for (const link of this.pageLinks) {
            const active = link.dataset.pageLink === page.id;
            link.classList.toggle("is-active", active);
            if (active) link.setAttribute("aria-current", "page");
            else link.removeAttribute("aria-current");
        }
        if (updateHash && page.id) {
            window.history.replaceState(null, "", `#${page.id}`);
        }
    }
}
