#!/usr/bin/env python3
"""Check local files and HTML anchors referenced by the built lore book."""

from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

BOOK = Path(__file__).resolve().parents[1] / "book"


class Page(HTMLParser):
    """Collect identifiers and local-link candidates from one HTML page."""

    def __init__(self, path: Path):
        super().__init__()
        self.links: list[str] = []
        self.ids: set[str] = set()
        self.feed(path.read_text(encoding="utf-8"))

    def handle_starttag(self, tag, attrs):
        """Record element identifiers, links, and embedded resources."""
        attrs = dict(attrs)
        if attrs.get("id"):
            self.ids.add(attrs["id"])
        self.links.extend(attrs[key] for key in ("href", "src") if attrs.get(key))


def check_book(root: Path) -> tuple[int, list[str]]:
    """Return the local-reference count and errors, excluding review artifacts."""
    root = root.resolve()
    if not (root / "index.html").is_file():
        return 0, ["Build the lore book first: missing index.html"]
    pages = {
        path: Page(path)
        for path in sorted(root.rglob("*.html"))
        if "review" not in path.relative_to(root).parts
    }
    checked, errors = 0, []
    for path, page in pages.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc:
                continue
            decoded = unquote(url.path)
            if decoded.startswith("/"):
                target = root / decoded.lstrip("/")
            elif decoded:
                target = path.parent / decoded
            else:
                target = path
            target = target.resolve()
            if target.is_dir():
                target = (target / "index.html").resolve()
            prefix = f"{path.relative_to(root)}: {link}"
            if not target.is_relative_to(root):
                errors.append(f"{prefix}: outside the book")
            elif not target.is_file():
                errors.append(f"{prefix}: missing file")
            elif url.fragment and target in pages:
                if unquote(url.fragment) not in pages[target].ids:
                    errors.append(f"{prefix}: missing HTML anchor")
            checked += 1
    return checked, errors


def main():
    """Exit unsuccessfully if built local links are broken."""
    checked, errors = check_book(BOOK)
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"Checked {checked} local links and assets; all files and HTML anchors exist.")


if __name__ == "__main__":
    main()
