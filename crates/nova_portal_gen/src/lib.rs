//! Core of the static mod-portal generator (task 20260715-142900).
//!
//! `generate` scans a SOURCE directory of mod folders (`webmods/` in the repo:
//! each subdirectory is one mod, the directory name is its id), validates what a
//! manifest-level gate can, and writes a deterministic portal tree:
//!
//! ```text
//! <out>/catalog.json                  # PortalCatalog (JSON, schema-versioned)
//! <out>/<id>/<version>/<files...>     # every file of the mod, verbatim copy
//! ```
//!
//! Validation here is the PUBLISH gate a manifest can support: the bundle
//! parses, the meta is publishable (non-empty name + version), every listed
//! content file AND declared resource exists, every `self://` asset ref in the
//! content names a declared resource, every `dep://<id>/` ref targets a declared
//! dependency (and, when that dependency is another portal mod, a declared
//! resource of it), ids are well-formed and unique (including against the SHIPPED
//! catalog, so a portal mod can never shadow an installed one), and declared
//! dependencies resolve within the portal + shipped set.
//! Content is parsed (to a `ron::Value`) only to walk its resource refs, never
//! LOADED - whether it actually loads against the bevy loaders is the
//! `webmods_validation` integration test's job (real loaders on PR CI), keeping
//! this binary engine-free and the deploy job fast.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use nova_mod_format::{
    BundleManifest, CatalogManifest, PortalCatalog, PortalEntry, PortalFile, PORTAL_SCHEMA_VERSION,
};
use sha2::{Digest, Sha256};

/// A validation or IO failure, with enough context to fix the offending mod.
#[derive(Debug)]
pub struct GenError(pub String);

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for GenError {}

fn err<T>(msg: impl Into<String>) -> Result<T, GenError> {
    Err(GenError(msg.into()))
}

/// Ids are directory names AND URL path segments: keep them boring.
fn validate_id(id: &str) -> Result<(), GenError> {
    let ok = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        err(format!(
            "mod id '{id}' is invalid: use lowercase ascii letters, digits and '-' only"
        ))
    }
}

/// The ids the SHIPPED catalog installs (a portal mod may not reuse one).
fn shipped_ids(shipped_catalog: &Path) -> Result<BTreeSet<String>, GenError> {
    let bytes = fs::read(shipped_catalog).map_err(|e| {
        GenError(format!(
            "cannot read shipped catalog {}: {e}",
            shipped_catalog.display()
        ))
    })?;
    let manifest: CatalogManifest = ron::de::from_bytes(&bytes).map_err(|e| {
        GenError(format!(
            "shipped catalog {} does not parse: {e}",
            shipped_catalog.display()
        ))
    })?;
    Ok(manifest.mods.into_iter().map(|m| m.id).collect())
}

/// Walk `dir` recursively and return every file's path RELATIVE to `dir`, with
/// forward slashes, sorted (deterministic output is part of the contract).
fn walk_files(dir: &Path) -> Result<Vec<PathBuf>, GenError> {
    fn inner(root: &Path, dir: &Path, acc: &mut Vec<PathBuf>) -> Result<(), GenError> {
        let entries = fs::read_dir(dir)
            .map_err(|e| GenError(format!("cannot read directory {}: {e}", dir.display())))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| GenError(format!("cannot read entry in {}: {e}", dir.display())))?;
            let path = entry.path();
            if path.is_dir() {
                inner(root, &path, acc)?;
            } else {
                acc.push(
                    path.strip_prefix(root)
                        .expect("walked path is under its root")
                        .to_path_buf(),
                );
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    inner(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

/// Collect every mod-relative (`self://`) asset FILE referenced anywhere in a
/// parsed content `ron::Value`, scheme- and label-stripped
/// (`self://models/hull.glb#Scene0` -> `models/hull.glb`). Comments are already
/// gone (this walks the PARSED value, not text), so a `self://` mentioned in a
/// content comment is not a false positive. Mirrors `nova_assets`'
/// `mod_refs::collect_self_refs`, but engine-free on `ron::Value`.
fn collect_self_refs(value: &ron::Value, out: &mut Vec<String>) {
    match value {
        ron::Value::String(s) => {
            if let Some(rest) = s.strip_prefix("self://") {
                out.push(rest.split('#').next().unwrap_or(rest).to_string());
            }
        }
        ron::Value::Seq(items) => items.iter().for_each(|v| collect_self_refs(v, out)),
        ron::Value::Map(map) => map.iter().for_each(|(_k, v)| collect_self_refs(v, out)),
        ron::Value::Option(Some(inner)) => collect_self_refs(inner, out),
        _ => {}
    }
}

/// A `dep://<id>/<path>` cross-mod ref found in content: the target dependency
/// id and the file (label-stripped), or a `dep://` leaf that is not
/// `dep://<id>/<path>` shaped. Mirrors `nova_assets`' `mod_refs` on `ron::Value`.
enum DepRef {
    Ref { id: String, file: String },
    Malformed(String),
}

/// Collect every `dep://<id>/<path>` ref in a parsed content `ron::Value`,
/// scheme-detected and (for the file) `#label`-stripped. Comments are already
/// gone (this walks the PARSED value), so a `dep://` in a comment is not a false
/// positive. Engine-free, like `collect_self_refs`.
fn collect_dep_refs(value: &ron::Value, out: &mut Vec<DepRef>) {
    match value {
        ron::Value::String(s) => {
            if let Some(rest) = s.strip_prefix("dep://") {
                match rest.split_once('/') {
                    Some((id, path)) if !id.is_empty() && !path.is_empty() => {
                        out.push(DepRef::Ref {
                            id: id.to_string(),
                            file: path.split('#').next().unwrap_or(path).to_string(),
                        })
                    }
                    _ => out.push(DepRef::Malformed(s.clone())),
                }
            }
        }
        ron::Value::Seq(items) => items.iter().for_each(|v| collect_dep_refs(v, out)),
        ron::Value::Map(map) => map.iter().for_each(|(_k, v)| collect_dep_refs(v, out)),
        ron::Value::Option(Some(inner)) => collect_dep_refs(inner, out),
        _ => {}
    }
}

/// Binary-asset extensions - mirror of `nova_assets::mod_refs::ASSET_EXTENSIONS`,
/// engine-free. A scheme-less content string ending in one of these is a bare
/// asset ref (the canonical model requires a `self://`/`dep://` scheme).
const ASSET_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "glb", "gltf", "ktx2", "exr", "hdr", "dds", "basis", "ogg", "wav", "mp3",
    "flac",
];

/// Collect every BARE (scheme-less) asset-path ref in a parsed content value - a
/// string ending in a known asset extension (after `#label` stripping) with no
/// `self://`/`dep://` scheme. Mirrors `nova_assets::mod_refs::bare_asset_refs`
/// on `ron::Value`; see it for why this is an extension heuristic.
fn collect_bare_refs(value: &ron::Value, out: &mut Vec<String>) {
    match value {
        ron::Value::String(s) => {
            if !s.starts_with("self://") && !s.starts_with("dep://") {
                let file = s.split('#').next().unwrap_or(s);
                if let Some((_, ext)) = file.rsplit_once('.') {
                    if ASSET_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
                        out.push(s.clone());
                    }
                }
            }
        }
        ron::Value::Seq(items) => items.iter().for_each(|v| collect_bare_refs(v, out)),
        ron::Value::Map(map) => map.iter().for_each(|(_k, v)| collect_bare_refs(v, out)),
        ron::Value::Option(Some(inner)) => collect_bare_refs(inner, out),
        _ => {}
    }
}

/// One `dep://` cross-mod ref a mod makes, kept for the cross-mod membership
/// check in [`generate`] (where every portal mod's resources are known).
struct DepUse {
    /// The content file the ref appears in (for the error message).
    content: String,
    /// The dependency id targeted.
    dep_id: String,
    /// The referenced file, label-stripped.
    file: String,
}

/// A validated portal entry plus the cross-mod data [`generate`] needs: the
/// mod's declared resources and every `dep://` ref it makes.
struct BuiltEntry {
    entry: PortalEntry,
    resources: Vec<String>,
    dep_refs: Vec<DepUse>,
}

/// Forward-slash string form of a relative path (portal paths are URL segments).
fn rel_str(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Validate one mod directory and build its catalog entry (no copying yet).
fn build_entry(mod_dir: &Path, id: &str) -> Result<BuiltEntry, GenError> {
    validate_id(id)?;

    // Exactly one *.bundle.ron at the mod root is the entry point.
    let bundles: Vec<PathBuf> = fs::read_dir(mod_dir)
        .map_err(|e| GenError(format!("cannot read mod dir {}: {e}", mod_dir.display())))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().ends_with(".bundle.ron"))
        })
        .collect();
    let bundle_path = match bundles.as_slice() {
        [one] => one.clone(),
        [] => return err(format!("mod '{id}': no *.bundle.ron at the mod root")),
        many => {
            return err(format!(
                "mod '{id}': expected exactly one *.bundle.ron at the mod root, found {}",
                many.len()
            ))
        }
    };

    let bytes = fs::read(&bundle_path)
        .map_err(|e| GenError(format!("mod '{id}': cannot read bundle manifest: {e}")))?;
    let manifest: BundleManifest = ron::de::from_bytes(&bytes)
        .map_err(|e| GenError(format!("mod '{id}': bundle manifest does not parse: {e}")))?;

    if manifest.meta.name.trim().is_empty() {
        return err(format!("mod '{id}': meta.name is required to publish"));
    }
    if manifest.meta.version.trim().is_empty() {
        return err(format!("mod '{id}': meta.version is required to publish"));
    }

    let mut files = Vec::new();
    let mut total_size = 0u64;
    for rel in walk_files(mod_dir)? {
        let abs = mod_dir.join(&rel);
        let bytes = fs::read(&abs)
            .map_err(|e| GenError(format!("mod '{id}': cannot read {}: {e}", abs.display())))?;
        let size = bytes.len() as u64;
        total_size += size;
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        files.push(PortalFile {
            path: rel_str(&rel),
            size,
            sha256,
        });
    }

    // Every declared content path must be a MEMBER of the walked file set - the
    // exact set the portal will serve. A plain existence check would accept an
    // escaping path (`../x.content.ron` joins outside the mod dir and may well
    // exist in the source tree) that is neither hashed, listed, nor copied,
    // publishing a mod whose entry point references a file the portal never
    // serves (review R1.1).
    let file_set: BTreeSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
    for content in &manifest.content {
        if !file_set.contains(content.as_str()) {
            return err(format!(
                "mod '{id}': listed content file '{content}' is not a file inside the mod \
                 directory (missing, escaping, or not slash-normalized)"
            ));
        }
    }

    // Declared binary resources (task 20260716-123544) get the SAME membership
    // gate as content: a `self://` content ref may only name a listed resource,
    // and the portal only serves files it walked+hashed. A resource that is
    // missing, escaping, or not slash-normalized would publish a mod whose
    // content points at a file the portal never serves.
    for resource in &manifest.resources {
        if !file_set.contains(resource.as_str()) {
            return err(format!(
                "mod '{id}': listed resource file '{resource}' is not a file inside the mod \
                 directory (missing, escaping, or not slash-normalized)"
            ));
        }
    }

    // The reverse membership: a `self://` asset ref in the content may only name
    // a DECLARED resource, and a `dep://<id>/` ref may only target a DECLARED
    // dependency. The runtime merge and the static content lint enforce this over
    // the repo tree; enforcing it here too closes the publish-time hole for a mod
    // published from OUTSIDE the repo (never repo-linted) - it cannot ship a
    // dangling ref. Content is parsed to a `ron::Value` (comments stripped, unlike
    // a text scan) and every resource-ref string leaf is checked; the generator
    // stays engine-free (no bevy, no typed `Content`).
    //
    // The `dep://` cross-mod resource membership (the file is one of `<id>`'s
    // declared resources) needs the OTHER mod's manifest, so it is deferred to
    // `generate` where every portal mod's resources are known; here we only pin
    // that `<id>` is a declared dependency of THIS mod (and not the implicit
    // `base`, whose files use a bare path).
    let resource_set: BTreeSet<&str> = manifest.resources.iter().map(|s| s.as_str()).collect();
    let declared_deps: BTreeSet<&str> = manifest
        .meta
        .dependencies
        .iter()
        .map(|s| s.as_str())
        .collect();
    let mut dep_refs = Vec::new();
    for content in &manifest.content {
        let text = fs::read_to_string(mod_dir.join(content))
            .map_err(|e| GenError(format!("mod '{id}': cannot read content '{content}': {e}")))?;
        let value: ron::Value = ron::de::from_str(&text).map_err(|e| {
            GenError(format!(
                "mod '{id}': content '{content}' does not parse: {e}"
            ))
        })?;
        let mut refs = Vec::new();
        collect_self_refs(&value, &mut refs);
        for file in refs {
            if !resource_set.contains(file.as_str()) {
                return err(format!(
                    "mod '{id}': content '{content}' references undeclared mod resource \
                     'self://{file}' - add it to the bundle manifest's `resources` list"
                ));
            }
        }
        let mut deps_used = Vec::new();
        collect_dep_refs(&value, &mut deps_used);
        for dep in deps_used {
            let (dep_id, file) = match dep {
                DepRef::Ref { id, file } => (id, file),
                DepRef::Malformed(raw) => {
                    return err(format!(
                        "mod '{id}': content '{content}' has a malformed dependency resource ref \
                         '{raw}' - expected 'dep://<id>/<path>'"
                    ))
                }
            };
            // `base` is the implicit universal dependency - `dep://base/<path>` is
            // always allowed (base need not be in `meta.dependencies`). Its
            // resource membership is not checked here: base is SHIPPED, so the
            // portal knows only its id, not its `resources` - backstopped by the
            // repo lint and the runtime gate (same as any shipped dependency).
            if dep_id != "base" && !declared_deps.contains(dep_id.as_str()) {
                return err(format!(
                    "mod '{id}': content '{content}' references resource 'dep://{dep_id}/{file}' \
                     but '{dep_id}' is not a declared dependency - add it to the bundle \
                     manifest's `meta.dependencies`"
                ));
            }
            dep_refs.push(DepUse {
                content: content.clone(),
                dep_id,
                file,
            });
        }
        // Canonical enforcement (task 20260717-002133): every asset ref must carry
        // a scheme. A bare (scheme-less) asset-path ref is rejected at publish -
        // the same gate the repo lint applies, closing the hole for a mod
        // published from outside the repo.
        let mut bare = Vec::new();
        collect_bare_refs(&value, &mut bare);
        if let Some(bare_ref) = bare.into_iter().next() {
            return err(format!(
                "mod '{id}': content '{content}' references asset '{bare_ref}' with no scheme - \
                 use 'self://{bare_ref}' (this mod's own art) or 'dep://<id>/{bare_ref}' (a \
                 dependency's, e.g. 'dep://base/{bare_ref}')"
            ));
        }
    }

    let resources = manifest.resources.clone();
    Ok(BuiltEntry {
        entry: PortalEntry {
            id: id.to_string(),
            version: manifest.meta.version.clone(),
            bundle: rel_str(
                bundle_path
                    .strip_prefix(mod_dir)
                    .expect("bundle path is under the mod dir"),
            ),
            meta: manifest.meta,
            files,
            total_size,
        },
        resources,
        dep_refs,
    })
}

/// Scan `source`, validate every mod (also against the shipped catalog's ids
/// when given), and write `catalog.json` + `<id>/<version>/<files>` under `out`.
/// Returns the generated catalog. Deterministic: entries sorted by id, files by
/// path, stable JSON field order.
pub fn generate(
    source: &Path,
    shipped_catalog: Option<&Path>,
    out: &Path,
) -> Result<PortalCatalog, GenError> {
    let shipped = match shipped_catalog {
        Some(path) => shipped_ids(path)?,
        None => BTreeSet::new(),
    };

    let mut mod_dirs: Vec<PathBuf> = fs::read_dir(source)
        .map_err(|e| GenError(format!("cannot read source {}: {e}", source.display())))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    mod_dirs.sort();

    let mut built = Vec::new();
    for mod_dir in &mod_dirs {
        let id = mod_dir
            .file_name()
            .expect("read_dir yields named entries")
            .to_string_lossy()
            .to_string();
        if shipped.contains(&id) {
            return err(format!(
                "mod '{id}' collides with a SHIPPED catalog id; portal mods must not shadow installed ones"
            ));
        }
        built.push(build_entry(mod_dir, &id)?);
    }
    // Subdirectory names are unique by the filesystem; sorted by the dir sort.
    // (A "duplicate id" case is therefore impossible by construction.)

    // Zero mods is a broken invocation (wrong --source path, bad checkout),
    // not an empty portal to publish silently (review R1.4).
    if built.is_empty() {
        return err(format!(
            "no mods found under {}; refusing to publish an empty portal",
            source.display()
        ));
    }

    // Dependencies must resolve within the portal + shipped set ('base' is
    // implicit and shipped, so declaring it also resolves).
    let portal_ids: BTreeSet<&str> = built.iter().map(|b| b.entry.id.as_str()).collect();
    for b in &built {
        for dep in &b.entry.meta.dependencies {
            if !portal_ids.contains(dep.as_str()) && !shipped.contains(dep) {
                return err(format!(
                    "mod '{}': dependency '{dep}' is neither a portal mod nor shipped",
                    b.entry.id
                ));
            }
        }
    }

    // Cross-mod resource membership: a `dep://<id>/<file>` ref may only name a
    // DECLARED resource of dependency `<id>`. When `<id>` is another PORTAL mod
    // its resources are known here, so the file is checked; when `<id>` is SHIPPED
    // only its id is known (the shipped catalog is a thin id list), so the
    // membership half is left to the runtime gate and the repo lint - the id is
    // still verified declared + resolvable above.
    let resources_by_id: BTreeMap<&str, BTreeSet<&str>> = built
        .iter()
        .map(|b| {
            (
                b.entry.id.as_str(),
                b.resources.iter().map(|s| s.as_str()).collect(),
            )
        })
        .collect();
    for b in &built {
        for dep in &b.dep_refs {
            if let Some(dep_resources) = resources_by_id.get(dep.dep_id.as_str()) {
                if !dep_resources.contains(dep.file.as_str()) {
                    return err(format!(
                        "mod '{}': content '{}' references undeclared resource \
                         'dep://{}/{}' of dependency '{}' - add it to that mod's `resources` list",
                        b.entry.id, dep.content, dep.dep_id, dep.file, dep.dep_id
                    ));
                }
            }
        }
    }

    let catalog = PortalCatalog {
        schema_version: PORTAL_SCHEMA_VERSION,
        entries: built.into_iter().map(|b| b.entry).collect(),
    };

    // Write the tree: files first, catalog last (a readable-but-incomplete
    // portal never lists a mod whose files are missing).
    for entry in &catalog.entries {
        let version_dir = out.join(&entry.id).join(&entry.version);
        for file in &entry.files {
            let src = source.join(&entry.id).join(&file.path);
            let dst = version_dir.join(&file.path);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| GenError(format!("cannot create {}: {e}", parent.display())))?;
            }
            fs::copy(&src, &dst).map_err(|e| {
                GenError(format!(
                    "cannot copy {} -> {}: {e}",
                    src.display(),
                    dst.display()
                ))
            })?;
        }
    }
    fs::create_dir_all(out)
        .map_err(|e| GenError(format!("cannot create {}: {e}", out.display())))?;
    let json = serde_json::to_string_pretty(&catalog)
        .map_err(|e| GenError(format!("cannot serialize catalog.json: {e}")))?;
    fs::write(out.join("catalog.json"), json.as_bytes())
        .map_err(|e| GenError(format!("cannot write catalog.json: {e}")))?;

    Ok(catalog)
}
