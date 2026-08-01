//! Where the portal lives and how that base URL is resolved per platform:
//! the [`PortalConfig`] resource plus the pure URL-derivation helpers behind
//! its wasm default (`window.location` -> sibling `mods` tree) and its
//! `?portal=` dev override.

use bevy::prelude::*;

/// The production portal base URL - the GitHub Pages tree `scripts/gen-portal.py`
/// publishes next to the wasm build (web/src/wiki/dev/mod-portal.md).
pub const DEFAULT_PORTAL_URL: &str = "https://alexjercan.github.io/nova-protocol/mods";

/// Where the portal lives: `<base_url>/catalog.json` +
/// `<base_url>/<id>/<version>/<files...>`.
///
/// Defaults per platform, resolved once at plugin build by
/// [`PortalConfig::from_environment`]:
/// - native: [`DEFAULT_PORTAL_URL`], overridable via the `NOVA_PORTAL_URL`
///   environment variable (dev/test builds point at localhost);
/// - wasm: derived from `window.location` (the game is served at
///   `<root>/play/`, the portal is its SIBLING `<root>/mods` - so a fork's
///   Pages deploy fetches its own portal with zero config), overridable via a
///   `?portal=<url>` query parameter.
#[derive(Resource, Clone, Debug)]
pub struct PortalConfig {
    /// The portal tree's base URL, no trailing slash required.
    pub base_url: String,
}

impl PortalConfig {
    /// Resolve the platform default + override chain described on the type.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_environment() -> Self {
        let base_url = std::env::var("NOVA_PORTAL_URL")
            .ok()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PORTAL_URL.to_string());
        Self { base_url }
    }

    /// Resolve the platform default + override chain described on the type.
    #[cfg(target_arch = "wasm32")]
    pub fn from_environment() -> Self {
        let window = web_sys::window();
        let base_url = window
            .as_ref()
            .map(|window| {
                let location = window.location();
                if let Some(url) =
                    portal_override_from_query(&location.search().unwrap_or_default())
                {
                    return url;
                }
                portal_base_from_href(&location.href().unwrap_or_default())
            })
            .unwrap_or_else(|| DEFAULT_PORTAL_URL.to_string());

        // Proactive cross-origin heads-up. A CORS failure reaches JS as an
        // opaque `TypeError: Failed to fetch` (indistinguishable from a refused
        // connection), so the post-failure error cannot name it - but comparing
        // the resolved base origin to the page origin is a reliable signal we
        // have BEFORE the fetch. Fires only when the portal is pointed
        // cross-origin (a `?portal=` to another host); the same-origin default
        // never trips it.
        if let Some(window) = window.as_ref() {
            if let (Ok(page_origin), Some(base_origin)) =
                (window.location().origin(), url_origin(&base_url))
            {
                if base_origin != page_origin {
                    warn!(
                        "portal: base '{base_url}' is cross-origin to the page ({page_origin}); \
                         the browser will block the catalog/file fetch unless the portal sends an \
                         Access-Control-Allow-Origin header. For local dev, serve the portal \
                         same-origin instead (see mod-portal.md, \"Local development\")."
                    );
                }
            }
        }

        Self { base_url }
    }

    /// The catalog's URL: `<base>/catalog.json`.
    pub fn catalog_url(&self) -> String {
        join_url(&self.base_url, "catalog.json")
    }

    /// One mod file's URL: `<base>/<id>/<version>/<path>` (the tree layout
    /// `scripts/gen-portal.py` writes).
    pub fn file_url(&self, id: &str, version: &str, path: &str) -> String {
        join_url(&self.base_url, &format!("{id}/{version}/{path}"))
    }
}

/// `<base>/<path>` with the trailing-slash seam normalized (a configured base
/// may or may not carry one).
fn join_url(base: &str, path: &str) -> String {
    format!("{}/{path}", base.trim_end_matches('/'))
}

/// The origin (`scheme://host[:port]`) of an absolute URL, or `None` when it
/// carries no `scheme://` (a relative base). Used to detect a cross-origin
/// portal config on wasm before the browser's opaque CORS failure. Pure and
/// cfg-independent for the native test pin, like the derivation fns below.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn url_origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    if host.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{host}"))
}

/// Derive the portal base from the page's own URL: drop the query/fragment
/// and any document file name, step out of a trailing `/play` segment (the
/// deploy serves the game at `<root>/play/`, the portal at `<root>/mods/`),
/// and append `mods`. Pure and cfg-independent ON PURPOSE: the only caller is
/// wasm, but the native unit tests are what pin its behavior.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn portal_base_from_href(href: &str) -> String {
    let trimmed = href.split(['#', '?']).next().unwrap_or(href);
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return format!("{}/mods", trimmed.trim_end_matches('/'));
    };
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let mut segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    // A final dotted segment is the document (index.html), not a directory.
    if segments.last().is_some_and(|s| s.contains('.')) {
        segments.pop();
    }
    if segments.last() == Some(&"play") {
        segments.pop();
    }
    segments.push("mods");
    format!("{scheme}://{host}/{}", segments.join("/"))
}

/// The `?portal=<url>` dev override, from a `Location::search` string
/// (`?a=b&portal=...`). Percent/plus-decoded; empty values do not override.
/// Cfg-independent like [`portal_base_from_href`], for the native test pin.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn portal_override_from_query(search: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .find_map(|pair| pair.strip_prefix("portal="))
        .map(percent_decode)
        .filter(|url| !url.is_empty())
}

/// Minimal application/x-www-form-urlencoded decode (`%XX` + `+` as space) -
/// enough for a URL-valued query parameter without a url-crate dependency.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    None => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_url_normalizes_the_slash_seam() {
        assert_eq!(
            join_url("http://x/mods", "catalog.json"),
            "http://x/mods/catalog.json"
        );
        assert_eq!(
            join_url("http://x/mods/", "catalog.json"),
            "http://x/mods/catalog.json"
        );
    }

    /// The wasm default derivation, pinned natively (the fn is pure): the
    /// game under `<root>/play/` fetches the SIBLING `<root>/mods`, documents
    /// and query/fragment noise are dropped, and a root-served page just
    /// appends `mods`.
    #[test]
    fn portal_base_derives_from_the_page_location() {
        assert_eq!(
            portal_base_from_href("https://alexjercan.github.io/nova-protocol/play/index.html"),
            "https://alexjercan.github.io/nova-protocol/mods"
        );
        assert_eq!(
            portal_base_from_href("https://alexjercan.github.io/nova-protocol/play/"),
            "https://alexjercan.github.io/nova-protocol/mods"
        );
        assert_eq!(
            portal_base_from_href("https://example.com/play/?seed=3#frag"),
            "https://example.com/mods"
        );
        assert_eq!(
            portal_base_from_href("http://localhost:8080/"),
            "http://localhost:8080/mods"
        );
        assert_eq!(
            portal_base_from_href("http://localhost:8080/index.html"),
            "http://localhost:8080/mods"
        );
    }

    /// The `?portal=` override wins over the location-derived default and is
    /// percent-decoded; other params and an empty value do not override.
    #[test]
    fn portal_query_override_parses() {
        assert_eq!(
            portal_override_from_query("?portal=http%3A%2F%2Flocalhost%3A8000%2Fmods"),
            Some("http://localhost:8000/mods".to_string())
        );
        assert_eq!(
            portal_override_from_query("?seed=1&portal=http://localhost:8000/mods"),
            Some("http://localhost:8000/mods".to_string())
        );
        assert_eq!(portal_override_from_query("?seed=1"), None);
        assert_eq!(portal_override_from_query("?portal="), None);
        assert_eq!(portal_override_from_query(""), None);
    }

    /// The cross-origin detector behind the wasm heads-up: origin is
    /// `scheme://host[:port]`, path/query/fragment dropped; a same-origin base
    /// matches the page origin, a different port/host does not; a relative base
    /// has no origin.
    #[test]
    fn url_origin_extracts_scheme_host_port() {
        assert_eq!(
            url_origin("http://localhost:8000/mods"),
            Some("http://localhost:8000".to_string())
        );
        assert_eq!(
            url_origin("https://alexjercan.github.io/nova-protocol/mods"),
            Some("https://alexjercan.github.io".to_string())
        );
        // Same host, different port is still cross-origin (the reported bug:
        // page :8090, portal :8000).
        assert_ne!(
            url_origin("http://localhost:8000/mods"),
            url_origin("http://localhost:8090/play/"),
        );
        // Same origin, different path -> equal origins (the same-origin default).
        assert_eq!(
            url_origin("http://localhost:8080/mods"),
            url_origin("http://localhost:8080/"),
        );
        // A relative base has no origin (never flagged cross-origin).
        assert_eq!(url_origin("/mods"), None);
        assert_eq!(url_origin("mods"), None);
    }
}
