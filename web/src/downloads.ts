// Landing-page progressive enhancement for the native download buttons.
//
// The three buttons in index.html (`.btn--download[data-os]`) ship with a
// static href to the GitHub releases/latest PAGE, so they work with JS
// disabled and survive any failure here. When JS runs we query the GitHub API
// for the newest release and rewrite each button's href to the exact per-OS
// asset, giving a one-click, correct-platform download that always tracks
// latest without a per-release edit to the page (see the task's DECISION.md).
//
// Asset matching is coupled to the filename convention in
// .github/workflows/release.yaml: `nova-protocol_v<VERSION>_<platform>.<ext>`.
// If that naming changes, the suffix table below must change with it.

const LATEST_RELEASE_API =
    "https://api.github.com/repos/alexjercan/nova-protocol/releases/latest";

type Os = "windows" | "macos" | "linux";

// data-os value -> the case-sensitive filename suffix of that platform's asset.
// `_web.zip` (the browser build) is deliberately absent so it is never matched.
const OS_ASSET_SUFFIX: Record<Os, string> = {
    windows: "_windows.zip",
    macos: "_macOS.dmg",
    linux: "_linux.tar.gz",
};

interface ReleaseAsset {
    name: string;
    browser_download_url: string;
}

// Pure: pick each platform's download URL out of a release's asset list by
// filename suffix. Exported for direct testing without a DOM or network - the
// project has no browser test runner, so this is the runtime-checkable core.
export function pickDownloadUrls(
    assets: readonly ReleaseAsset[]
): Partial<Record<Os, string>> {
    const urls: Partial<Record<Os, string>> = {};
    for (const os of Object.keys(OS_ASSET_SUFFIX) as Os[]) {
        const suffix = OS_ASSET_SUFFIX[os];
        const match = assets.find((a) => a.name.endsWith(suffix));
        if (match) {
            urls[os] = match.browser_download_url;
        }
    }
    return urls;
}

// Fetch the latest release and deep-link each button. Any failure (network,
// rate limit, unexpected shape, missing asset) leaves the static
// releases/latest fallback in place - the buttons are never worse off.
export async function enhanceDownloadButtons(): Promise<void> {
    const buttons = Array.from(
        document.querySelectorAll<HTMLAnchorElement>(".btn--download[data-os]")
    );
    if (buttons.length === 0) {
        return;
    }

    let urls: Partial<Record<Os, string>>;
    try {
        const res = await fetch(LATEST_RELEASE_API, {
            headers: { Accept: "application/vnd.github+json" },
        });
        if (!res.ok) {
            return;
        }
        const release = (await res.json()) as { assets?: ReleaseAsset[] };
        if (!Array.isArray(release.assets)) {
            return;
        }
        urls = pickDownloadUrls(release.assets);
    } catch {
        return;
    }

    for (const button of buttons) {
        const os = button.dataset.os as Os | undefined;
        const url = os ? urls[os] : undefined;
        if (url) {
            button.href = url;
        }
    }
}
