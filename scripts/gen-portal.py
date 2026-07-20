#!/usr/bin/env python3
"""The static mod-portal generator (the deploy workflow + preview script run this).

Interim build-time tool (task 20260718-152247): a hosted mods API will eventually
replace it wholesale. It scans a SOURCE directory of mod folders (`webmods/` in
the repo: each subdirectory is one mod, the directory name is its id), runs the
manifest-level PUBLISH gates, and writes a deterministic portal tree:

    <out>/catalog.json                  # PortalCatalog (JSON, schema-versioned)
    <out>/<id>/<version>/<files...>     # every file of the mod, verbatim copy

Originally a byte-for-byte port of the now-removed Rust `nova_portal_gen` crate
(task 20260720-230924 retired the crate once this was proven at parity). Stdlib
only. Its publish gates are exercised by
`crates/nova_assets/tests/gen_portal_gate.rs`.

The PUBLISH gates (each rejects with a non-zero exit):
  - the shipped catalog parses; a portal id may not collide with a shipped id
  - the source has at least one mod (refuse to publish an empty portal)
  - each mod has exactly one *.bundle.ron at its root and it parses
  - meta.name and meta.version are non-empty (publishable meta)
  - every listed content file AND declared resource is a MEMBER of the walked
    file set (not missing / escaping / un-normalized)
  - every content parses; every `self://` ref names a declared resource; every
    `dep://<id>/` ref targets a declared dependency (base is implicit); a
    malformed `dep://` or a bare (scheme-less) asset ref is rejected
  - declared dependencies resolve within the portal + shipped set
  - a `dep://<id>/<file>` where `<id>` is another PORTAL mod names one of its
    declared resources

Content is parsed to a value tree (comments stripped) only to walk its string
leaves for asset refs - never LOADED (engine-free, no bevy).
"""

import argparse
import hashlib
import json
import shutil
import sys
from pathlib import Path


PORTAL_SCHEMA_VERSION = 1

# Mirror of nova_assets::mod_refs::ASSET_EXTENSIONS. A
# scheme-less content string ending in one of these is a bare asset ref.
ASSET_EXTENSIONS = frozenset(
    [
        "png",
        "jpg",
        "jpeg",
        "glb",
        "gltf",
        "ktx2",
        "exr",
        "hdr",
        "dds",
        "basis",
        "ogg",
        "wav",
        "mp3",
        "flac",
    ]
)


class GenError(Exception):
    """A validation or IO failure, with enough context to fix the offending mod."""


# ---------------------------------------------------------------------------
# Minimal RON reader.
#
# We only need a value TREE (to walk string leaves) plus structured access to a
# handful of manifest fields. RON's full grammar is large; this parser covers the
# subset the manifests and content files actually use: comments (// and /* */),
# strings (with escapes) and raw strings, chars, numbers, bools, unit/None,
# Some(x), sequences [ ], maps { }, tuples ( ), named structs `Name( k: v, ... )`
# and named tuples / enum variants `Name(...)` / `Name`. Named structs become
# dicts (string keys); unnamed tuples and enum-variant payloads become lists;
# this matches how `ron::Value` flattens things for our ref walk (which only ever
# inspects String leaves, Seq and Map children).
# ---------------------------------------------------------------------------


class _RonParser:
    def __init__(self, text):
        self.s = text
        self.i = 0
        self.n = len(text)

    def error(self, msg):
        # Approximate the offset with a 1-based char index for a readable message.
        raise GenError(f"{msg} at char {self.i}")

    def parse(self):
        self._ws()
        value = self._value()
        self._ws()
        if self.i != self.n:
            self.error("trailing data after top-level value")
        return value

    def _ws(self):
        while self.i < self.n:
            c = self.s[self.i]
            if c in " \t\r\n":
                self.i += 1
            elif c == "/" and self.i + 1 < self.n and self.s[self.i + 1] == "/":
                self.i += 2
                while self.i < self.n and self.s[self.i] != "\n":
                    self.i += 1
            elif c == "/" and self.i + 1 < self.n and self.s[self.i + 1] == "*":
                self.i += 2
                depth = 1
                while self.i < self.n and depth > 0:
                    if self.s.startswith("/*", self.i):
                        depth += 1
                        self.i += 2
                    elif self.s.startswith("*/", self.i):
                        depth -= 1
                        self.i += 2
                    else:
                        self.i += 1
            else:
                break

    def _peek(self):
        return self.s[self.i] if self.i < self.n else ""

    def _value(self):
        self._ws()
        c = self._peek()
        if c == "":
            self.error("unexpected end of input")
        if c == '"':
            return self._string()
        if c == "r" and self._is_raw_string():
            return self._raw_string()
        if c == "'":
            return self._char()
        if c == "[":
            return self._seq()
        if c == "{":
            return self._map()
        if c == "(":
            return self._tuple_or_struct()
        if c == "-" or c.isdigit() or c == "+" or c == ".":
            return self._number()
        # An identifier: bool, None, Some(...), a named struct/tuple, or a bare
        # enum-unit variant.
        ident = self._ident()
        if ident == "true":
            return True
        if ident == "false":
            return False
        if ident == "None":
            return None
        if ident == "Some":
            self._ws()
            if self._peek() != "(":
                self.error("expected '(' after Some")
            self.i += 1
            inner = self._value()
            self._ws()
            if self._peek() != ")":
                self.error("expected ')' to close Some(...)")
            self.i += 1
            return inner
        # A named construct: `Name( ... )` or a bare unit variant `Name`.
        self._ws()
        if self._peek() == "(":
            return self._tuple_or_struct(named=ident)
        # Bare identifier / unit enum variant: a scalar leaf we do not walk into.
        return _Unit(ident)

    def _is_raw_string(self):
        # r"..." or r#"..."#
        j = self.i + 1
        while j < self.n and self.s[j] == "#":
            j += 1
        return j < self.n and self.s[j] == '"'

    def _raw_string(self):
        self.i += 1  # 'r'
        hashes = 0
        while self._peek() == "#":
            hashes += 1
            self.i += 1
        if self._peek() != '"':
            self.error("malformed raw string")
        self.i += 1
        terminator = '"' + "#" * hashes
        end = self.s.find(terminator, self.i)
        if end == -1:
            self.error("unterminated raw string")
        out = self.s[self.i : end]
        self.i = end + len(terminator)
        return out

    def _string(self):
        self.i += 1  # opening quote
        out = []
        while self.i < self.n:
            c = self.s[self.i]
            if c == '"':
                self.i += 1
                return "".join(out)
            if c == "\\":
                self.i += 1
                e = self.s[self.i] if self.i < self.n else ""
                if e == "n":
                    out.append("\n")
                elif e == "t":
                    out.append("\t")
                elif e == "r":
                    out.append("\r")
                elif e == "\\":
                    out.append("\\")
                elif e == '"':
                    out.append('"')
                elif e == "'":
                    out.append("'")
                elif e == "0":
                    out.append("\0")
                elif e == "u":
                    # \u{XXXX}
                    if self.s[self.i + 1] == "{":
                        end = self.s.find("}", self.i)
                        hexv = self.s[self.i + 2 : end]
                        out.append(chr(int(hexv, 16)))
                        self.i = end
                    else:
                        self.error("malformed \\u escape")
                elif e == "x":
                    hexv = self.s[self.i + 1 : self.i + 3]
                    out.append(chr(int(hexv, 16)))
                    self.i += 2
                else:
                    out.append(e)
                self.i += 1
            else:
                out.append(c)
                self.i += 1
        self.error("unterminated string")

    def _char(self):
        self.i += 1  # opening '
        if self._peek() == "\\":
            self.i += 1
            self.i += 1  # skip escaped char (chars are scalar leaves we ignore)
        else:
            self.i += 1
        if self._peek() != "'":
            self.error("unterminated char")
        self.i += 1
        return _Unit("char")

    def _number(self):
        start = self.i
        if self._peek() in "+-":
            self.i += 1
        while self.i < self.n and (
            self.s[self.i].isdigit()
            or self.s[self.i] in "._eE+-xXaAbBcCdDfF"
        ):
            self.i += 1
        return _Unit(self.s[start : self.i])

    def _ident(self):
        start = self.i
        while self.i < self.n and (self.s[self.i].isalnum() or self.s[self.i] == "_"):
            self.i += 1
        if self.i == start:
            self.error("expected identifier")
        return self.s[start : self.i]

    def _seq(self):
        self.i += 1  # [
        items = []
        while True:
            self._ws()
            if self._peek() == "]":
                self.i += 1
                return items
            items.append(self._value())
            self._ws()
            if self._peek() == ",":
                self.i += 1
            elif self._peek() == "]":
                self.i += 1
                return items
            else:
                self.error("expected ',' or ']' in sequence")

    def _map(self):
        self.i += 1  # {
        out = {}
        while True:
            self._ws()
            if self._peek() == "}":
                self.i += 1
                return out
            key = self._value()
            self._ws()
            if self._peek() != ":":
                self.error("expected ':' in map")
            self.i += 1
            val = self._value()
            out[_as_key(key)] = val
            self._ws()
            if self._peek() == ",":
                self.i += 1
            elif self._peek() == "}":
                self.i += 1
                return out
            else:
                self.error("expected ',' or '}' in map")

    def _tuple_or_struct(self, named=None):
        # At '('. Could be a struct `( k: v, ... )`, a tuple `( a, b )`, or a
        # named variant. We disambiguate by looking for `ident :` at the head.
        self.i += 1  # (
        self._ws()
        if self._peek() == ")":
            self.i += 1
            return {} if self._looks_struct_empty(named) else []
        if self._is_struct_head():
            return self._struct_body()
        return self._tuple_body()

    def _looks_struct_empty(self, named):
        # An empty `()` is a unit tuple; treat as empty list (no leaves).
        return False

    def _is_struct_head(self):
        # Save position, try to read `<key> :` where key is a bare ident or a
        # plain "..." string (raw-string keys do not occur in webmods manifests
        # and are not handled here).
        save = self.i
        try:
            self._ws()
            c = self._peek()
            if c == '"':
                self._string()
            elif c.isalpha() or c == "_":
                self._ident()
            else:
                self.i = save
                return False
            self._ws()
            is_colon = self._peek() == ":"
            self.i = save
            return is_colon
        except GenError:
            self.i = save
            return False

    def _struct_body(self):
        out = {}
        while True:
            self._ws()
            if self._peek() == ")":
                self.i += 1
                return out
            c = self._peek()
            if c == '"':
                key = self._string()
            else:
                key = self._ident()
            self._ws()
            if self._peek() != ":":
                self.error("expected ':' in struct")
            self.i += 1
            val = self._value()
            out[key] = val
            self._ws()
            if self._peek() == ",":
                self.i += 1
            elif self._peek() == ")":
                self.i += 1
                return out
            else:
                self.error("expected ',' or ')' in struct")

    def _tuple_body(self):
        items = []
        while True:
            self._ws()
            if self._peek() == ")":
                self.i += 1
                return items
            items.append(self._value())
            self._ws()
            if self._peek() == ",":
                self.i += 1
            elif self._peek() == ")":
                self.i += 1
                return items
            else:
                self.error("expected ',' or ')' in tuple")


class _Unit:
    """A scalar leaf we never walk into (number, char, bare enum variant)."""

    __slots__ = ("raw",)

    def __init__(self, raw):
        self.raw = raw


def _as_key(value):
    if isinstance(value, str):
        return value
    if isinstance(value, _Unit):
        return value.raw
    return value


def ron_parse(text):
    return _RonParser(text).parse()


# ---------------------------------------------------------------------------
# Ref walking (mirrors nova_assets::mod_refs's collect_self_refs / collect_dep_refs /
# collect_bare_refs over a parsed value tree - String leaves only).
# ---------------------------------------------------------------------------


def _walk_strings(value):
    """Yield every String leaf in the parsed value tree, in traversal order."""
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from _walk_strings(item)
    elif isinstance(value, dict):
        for v in value.values():
            yield from _walk_strings(v)
    # None (Option::None), bool, _Unit: no string leaves.


def collect_self_refs(value):
    out = []
    for s in _walk_strings(value):
        if s.startswith("self://"):
            rest = s[len("self://") :]
            out.append(rest.split("#", 1)[0])
    return out


def collect_dep_refs(value):
    """Return (refs, malformed): refs is [(id, file)], malformed is [raw]."""
    refs = []
    malformed = []
    for s in _walk_strings(value):
        if s.startswith("dep://"):
            rest = s[len("dep://") :]
            head, sep, tail = rest.partition("/")
            if sep and head and tail:
                refs.append((head, tail.split("#", 1)[0]))
            else:
                malformed.append(s)
    return refs, malformed


def collect_bare_refs(value):
    out = []
    for s in _walk_strings(value):
        if s.startswith("self://") or s.startswith("dep://"):
            continue
        file = s.split("#", 1)[0]
        if "." in file:
            ext = file.rsplit(".", 1)[1]
            if ext.lower() in ASSET_EXTENSIONS:
                out.append(s)
    return out


# ---------------------------------------------------------------------------
# Generator.
# ---------------------------------------------------------------------------


def validate_id(mod_id):
    ok = bool(mod_id) and all(
        ("a" <= c <= "z") or ("0" <= c <= "9") or c == "-" for c in mod_id
    )
    if not ok:
        raise GenError(
            f"mod id '{mod_id}' is invalid: use lowercase ascii letters, digits and '-' only"
        )


def shipped_ids(shipped_catalog):
    try:
        text = shipped_catalog.read_text(encoding="utf-8")
    except OSError as e:
        raise GenError(f"cannot read shipped catalog {shipped_catalog}: {e}")
    try:
        manifest = ron_parse(text)
    except GenError as e:
        raise GenError(f"shipped catalog {shipped_catalog} does not parse: {e}")
    ids = set()
    for entry in manifest.get("mods", []):
        ids.add(entry["id"])
    return ids


def walk_files(directory):
    """Every file path RELATIVE to `directory`, forward slashes, sorted.

    Matches the Rust `walk_files`: recursion order does not matter because the
    result is sorted; sort is over the OS-path form (identical to the string form
    here since components are joined by '/')."""
    files = []
    for path in directory.rglob("*"):
        if path.is_file():
            files.append(path.relative_to(directory))
    # Sort by PathBuf ordering == component-wise; str with '/' separators gives
    # the same order for these inputs. Sort on the parts tuple to match PathBuf.
    files.sort(key=lambda p: p.parts)
    return files


def rel_str(path):
    return "/".join(path.parts)


def build_entry(mod_dir, mod_id):
    validate_id(mod_id)

    bundles = [
        p
        for p in mod_dir.iterdir()
        if p.is_file() and p.name.endswith(".bundle.ron")
    ]
    if len(bundles) == 1:
        bundle_path = bundles[0]
    elif len(bundles) == 0:
        raise GenError(f"mod '{mod_id}': no *.bundle.ron at the mod root")
    else:
        raise GenError(
            f"mod '{mod_id}': expected exactly one *.bundle.ron at the mod root, "
            f"found {len(bundles)}"
        )

    try:
        manifest_text = bundle_path.read_text(encoding="utf-8")
    except OSError as e:
        raise GenError(f"mod '{mod_id}': cannot read bundle manifest: {e}")
    try:
        manifest = ron_parse(manifest_text)
    except GenError as e:
        raise GenError(f"mod '{mod_id}': bundle manifest does not parse: {e}")

    meta = manifest.get("meta", {}) or {}
    content_list = manifest.get("content", []) or []
    resources = manifest.get("resources", []) or []

    name = meta.get("name", "") or ""
    version = meta.get("version", "") or ""
    if name.strip() == "":
        raise GenError(f"mod '{mod_id}': meta.name is required to publish")
    if version.strip() == "":
        raise GenError(f"mod '{mod_id}': meta.version is required to publish")

    files = []
    total_size = 0
    for rel in walk_files(mod_dir):
        abs_path = mod_dir / rel
        try:
            data = abs_path.read_bytes()
        except OSError as e:
            raise GenError(f"mod '{mod_id}': cannot read {abs_path}: {e}")
        size = len(data)
        total_size += size
        sha256 = hashlib.sha256(data).hexdigest()
        files.append({"path": rel_str(rel), "size": size, "sha256": sha256})

    file_set = {f["path"] for f in files}
    for content in content_list:
        if content not in file_set:
            raise GenError(
                f"mod '{mod_id}': listed content file '{content}' is not a file inside "
                f"the mod directory (missing, escaping, or not slash-normalized)"
            )
    for resource in resources:
        if resource not in file_set:
            raise GenError(
                f"mod '{mod_id}': listed resource file '{resource}' is not a file inside "
                f"the mod directory (missing, escaping, or not slash-normalized)"
            )

    resource_set = set(resources)
    declared_deps = set(meta.get("dependencies", []) or [])
    dep_refs = []
    for content in content_list:
        try:
            content_text = (mod_dir / content).read_text(encoding="utf-8")
        except OSError as e:
            raise GenError(f"mod '{mod_id}': cannot read content '{content}': {e}")
        try:
            value = ron_parse(content_text)
        except GenError as e:
            raise GenError(f"mod '{mod_id}': content '{content}' does not parse: {e}")

        for file in collect_self_refs(value):
            if file not in resource_set:
                raise GenError(
                    f"mod '{mod_id}': content '{content}' references undeclared mod resource "
                    f"'self://{file}' - add it to the bundle manifest's `resources` list"
                )

        refs, malformed = collect_dep_refs(value)
        if malformed:
            raise GenError(
                f"mod '{mod_id}': content '{content}' has a malformed dependency resource ref "
                f"'{malformed[0]}' - expected 'dep://<id>/<path>'"
            )
        for dep_id, file in refs:
            if dep_id != "base" and dep_id not in declared_deps:
                raise GenError(
                    f"mod '{mod_id}': content '{content}' references resource "
                    f"'dep://{dep_id}/{file}' but '{dep_id}' is not a declared dependency - "
                    f"add it to the bundle manifest's `meta.dependencies`"
                )
            dep_refs.append({"content": content, "dep_id": dep_id, "file": file})

        bare = collect_bare_refs(value)
        if bare:
            bare_ref = bare[0]
            raise GenError(
                f"mod '{mod_id}': content '{content}' references asset '{bare_ref}' with no "
                f"scheme - use 'self://{bare_ref}' (this mod's own art) or "
                f"'dep://<id>/{bare_ref}' (a dependency's, e.g. 'dep://base/{bare_ref}')"
            )

    # Build the meta dict in ModMeta struct-declaration order (byte-for-byte JSON).
    entry_meta = {
        "name": name,
        "description": meta.get("description", "") or "",
        "author": meta.get("author", "") or "",
        "version": version,
        "dependencies": list(meta.get("dependencies", []) or []),
        "icon": meta.get("icon", None),
        "screenshots": list(meta.get("screenshots", []) or []),
    }
    entry = {
        "id": mod_id,
        "version": version,
        "bundle": rel_str(bundle_path.relative_to(mod_dir)),
        "meta": entry_meta,
        "files": files,
        "total_size": total_size,
    }
    return {"entry": entry, "resources": list(resources), "dep_refs": dep_refs}


def generate(source, shipped_catalog, out):
    shipped = shipped_ids(shipped_catalog) if shipped_catalog is not None else set()

    mod_dirs = sorted(
        (p for p in source.iterdir() if p.is_dir()), key=lambda p: p.parts
    )

    built = []
    for mod_dir in mod_dirs:
        mod_id = mod_dir.name
        if mod_id in shipped:
            raise GenError(
                f"mod '{mod_id}' collides with a SHIPPED catalog id; portal mods must not "
                f"shadow installed ones"
            )
        built.append(build_entry(mod_dir, mod_id))

    if not built:
        raise GenError(
            f"no mods found under {source}; refusing to publish an empty portal"
        )

    portal_ids = {b["entry"]["id"] for b in built}
    for b in built:
        for dep in b["entry"]["meta"]["dependencies"]:
            if dep not in portal_ids and dep not in shipped:
                raise GenError(
                    f"mod '{b['entry']['id']}': dependency '{dep}' is neither a portal mod "
                    f"nor shipped"
                )

    resources_by_id = {b["entry"]["id"]: set(b["resources"]) for b in built}
    for b in built:
        for dep in b["dep_refs"]:
            dep_resources = resources_by_id.get(dep["dep_id"])
            if dep_resources is not None and dep["file"] not in dep_resources:
                raise GenError(
                    f"mod '{b['entry']['id']}': content '{dep['content']}' references "
                    f"undeclared resource 'dep://{dep['dep_id']}/{dep['file']}' of dependency "
                    f"'{dep['dep_id']}' - add it to that mod's `resources` list"
                )

    catalog = {
        "schema_version": PORTAL_SCHEMA_VERSION,
        "entries": [b["entry"] for b in built],
    }

    # Write files first, catalog last (a readable-but-incomplete portal never
    # lists a mod whose files are missing).
    for entry in catalog["entries"]:
        version_dir = out / entry["id"] / entry["version"]
        for file in entry["files"]:
            src = source / entry["id"] / file["path"]
            dst = version_dir / file["path"]
            dst.parent.mkdir(parents=True, exist_ok=True)
            try:
                shutil.copyfile(src, dst)
            except OSError as e:
                raise GenError(f"cannot copy {src} -> {dst}: {e}")

    out.mkdir(parents=True, exist_ok=True)
    # serde_json::to_string_pretty parity: 2-space indent, ": " after keys, no
    # trailing spaces, no trailing newline, non-ASCII left unescaped.
    text = json.dumps(catalog, indent=2, ensure_ascii=False)
    (out / "catalog.json").write_text(text, encoding="utf-8")

    return catalog


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="gen-portal.py",
        description=(
            "Generate the static mod portal (catalog.json + hashed file copies) "
            "from a webmods/ source tree"
        ),
    )
    parser.add_argument(
        "--source",
        required=True,
        type=Path,
        help="Directory of mod sources: each subdirectory is one mod (dir name = id).",
    )
    parser.add_argument(
        "--shipped",
        type=Path,
        default=None,
        help="The game's shipped mods.catalog.ron; portal ids must not collide with it.",
    )
    parser.add_argument(
        "--out",
        required=True,
        type=Path,
        help="Output directory for catalog.json + <id>/<version>/<files>.",
    )
    args = parser.parse_args(argv)

    try:
        catalog = generate(args.source, args.shipped, args.out)
    except GenError as e:
        print(f"portal generation failed: {e}", file=sys.stderr)
        return 1

    total = sum(e["total_size"] for e in catalog["entries"])
    print(
        f"portal: published {len(catalog['entries'])} mod(s), {total} bytes -> {args.out}"
    )
    for entry in catalog["entries"]:
        print(
            f"  {entry['id']} {entry['version']} "
            f"({len(entry['files'])} files, {entry['total_size']} bytes)"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
